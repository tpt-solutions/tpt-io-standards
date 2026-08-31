# tpt-io-netcdf

A `no_std`-friendly, dependency-light parser for **NetCDF-3 classic / 64-bit
offset** files (dimensions, variables, global and per-variable attributes),
plus **NetCDF-4 (HDF5)** format detection and superblock triage. Part of the
[`tpt-io-standards`](https://github.com/tpt-solutions/tpt-io-standards)
workspace.

## NetCDF-3 usage

```rust
use tpt_io_core::Reader;
use tpt_io_netcdf::header::{NcType, NetCdf};
use tpt_io_netcdf::model::NetCdf as _;
use tpt_io_netcdf::parser::NetCdfReader;

let bytes: Vec<u8> = std::fs::read("sample.nc").unwrap();
let nc: NetCdf = NetCdfReader::new().parse(&bytes).unwrap();

// Dimensions, variables, and attributes.
let lat = nc.dim("lat").unwrap();
let temp = nc.var("temp").unwrap();
let units = temp.attrs.iter().find(|a| a.name == "units");

// Coordinate variables map an index axis to world coordinates.
let coords: Vec<&str> = nc.coordinate_variables().iter().map(|v| v.name.as_str()).collect();

// Read a hyperslab without materializing the whole array — ideal for
// `time × depth × lat × lon` grids. Last dimension varies fastest.
let slab = nc.read_hyperslab(&bytes, temp, &[0, 0, 0], &[1, lat.len as usize, 4]).unwrap();
```

### Record (unlimited) dimensions

A variable whose first dimension is the unlimited (record) dimension stores its
records interleaved across all record variables on disk, so a single contiguous
`var_data` slice is not meaningful. Use `NetCdf::read_hyperslab` (which handles
the record layout) or `NetCdf::is_record_var` to detect such variables.

## NetCDF-4 (HDF5)

NetCDF-4 files are HDF5 containers. The crate detects them, parses the
superblock, descends the object tree to enumerate datasets, reads their
dimensions, types, storage layout, and filter pipeline (deflate/shuffle),
and can read back both contiguous and chunked+filtered data:

```rust
use tpt_io_netcdf::netcdf4::{detect_format, parse_hdf5_superblock, FileKind, parse};

let kind = detect_format(&bytes);          // Some(FileKind::NetCdf4)
if kind == Some(FileKind::NetCdf4) {
    let f = parse(&bytes).expect("parse netcdf4");
    for ds in &f.datasets {
        println!("{}: dims={:?}, type={:?}", ds.name, ds.dims, ds.nc_type);
        let raw = ds.read_data(&bytes).expect("read data");
    }
}
```

## Features

| Feature | Default | Effect |
| --- | --- | --- |
| `std` | yes | Standard library support |
| `alloc` | no | `alloc`-based types without `std` |
| `sci-grid-integration` | no | Stub conversion boundary for future `tpt-sci-grid` crate |

## Testing

```sh
cargo test -p tpt-io-netcdf
cargo +nightly fuzz run netcdf
```

## Status

Per the workspace `todo.md`, `tpt-io-netcdf` implements NetCDF-3 classic/64-bit
offset parsing, NetCDF-4 (HDF5) detection and dataset reading, chunked hyperslab
reading, header round-trip, fuzz, golden-file, and property-based tests. `cargo
publish --dry-run` is blocked until `tpt-io-core` is on crates.io (see §4 publish
order).

## License

Licensed under either of [MIT](../LICENSE-MIT) or [Apache-2.0](../LICENSE-APACHE)
at your option.
