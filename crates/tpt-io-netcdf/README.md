# tpt-io-netcdf

A `no_std`-friendly, dependency-light parser for **NetCDF-3 classic / 64-bit
offset** files (dimensions, variables, global and per-variable attributes),
plus **NetCDF-4 (HDF5)** format detection and superblock triage. Part of the
[`tpt-io-standards`](https://github.com/REPLACE_ME_ORG/tpt-io-standards)
workspace.

## Status

| Capability | Status |
| --- | --- |
| NetCDF-3 classic / 64-bit offset header parsing | ✅ |
| Dimension extraction (incl. unlimited/record dims) | ✅ |
| Variable metadata (attrs: `units`, `long_name`, `valid_range` as raw `Attr`s) | ✅ |
| Global attribute extraction | ✅ |
| Coordinate-variable detection (`NetCdf::coordinate_variables`) | ✅ |
| Chunked / hyperslab reading of large multi-dim arrays | ✅ (`NetCdf::read_hyperslab`) |
| Header round-trip (`writer::round_trip`) | ✅ |
| `tpt-sci-grid` integration (feature-gated stub) | ✅ (`sci-grid-integration`) |
| NetCDF-4 (HDF5) detection + superblock parse | ✅ |
| NetCDF-4 dataset / deflate / shuffle / chunking decode | ✅ |
| Fuzz target, golden corpus, proptest | ✅ |

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

- `std` (default) / `alloc` — `no_std` support via `alloc`.
- `sci-grid-integration` — exposes a stub conversion boundary for the future
  `tpt-sci-grid` crate (`tpt_io_netcdf::integration::sci_grid::to_grid`).

## License

Licensed under either of MIT or Apache-2.0 at your option.
