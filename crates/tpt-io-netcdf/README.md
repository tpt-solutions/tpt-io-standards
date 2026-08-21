# tpt-io-netcdf

NetCDF-3 classic / 64-bit parser for the `tpt-io` workspace, targeting
climate, ocean, and atmospheric science data.

The crate parses a NetCDF-3 byte stream into a typed [`NetCdf`] model —
dimensions, global attributes, and variable definitions with their header
offsets — and round-trips the header back to bytes via a
[`tpt_io_core::Writer`]. Per-variable data lives in the raw file bytes and is
addressed through the offset/size helpers on [`NetCdf`].

[`NetCdf`]: https://docs.rs/tpt-io-netcdf/latest/tpt_io_netcdf/header/struct.NetCdf.html
[`tpt_io_core::Writer`]: https://docs.rs/tpt-io-core/latest/tpt_io_core/trait.Writer.html

> **Scope:** This crate implements the **NetCDF-3 classic and 64-bit offset**
> formats. NetCDF-4 (HDF5-based) files, chunked/packed data sections, and
> deflate/shuffle compression are **not** yet supported (see `todo.md`).

## Quick start

```rust
use tpt_io_core::Reader;
use tpt_io_netcdf::parser::NetCdfReader;
use tpt_io_netcdf::writer::serialize_header;
use tpt_io_netcdf::header::{Dim, NetCdf, NcType, Var};

let nc = NetCdf {
    version: 1,
    numrecs: Some(1),
    dims: vec![Dim { name: "time".into(), len: 1 }],
    global_attrs: vec![],
    vars: vec![Var {
        name: "temp".into(),
        nc_type: NcType::Float,
        dim_ids: vec![0],
        attrs: vec![],
        vlen: 4,
        begin: 0,
    }],
    header_len: 0,
};
let bytes = serialize_header(&nc);
let parsed = NetCdfReader::new().parse(&bytes).unwrap();
assert_eq!(parsed.dims[0].name, "time");
```

## Features

| Feature                | Default | Description                                              |
| ---------------------- | ------- | -------------------------------------------------------- |
| `std`                  | yes     | Standard-library support (`Vec`, etc.).                  |
| `alloc`                | no      | `no_std` + `alloc` only.                                 |
| `sci-grid-integration` | no      | Stub conversion boundary for the future `tpt-sci-grid`.  |

## Validation

[`NetCdfValidator`] checks that every variable references a defined dimension
(syntactic) and that no variable uses an unknown type code (semantic). It is
run automatically by [`NetCdfReader`].

[`NetCdfValidator`]: https://docs.rs/tpt-io-netcdf/latest/tpt_io_netcdf/parser/struct.NetCdfValidator.html
[`NetCdfReader`]: https://docs.rs/tpt-io-netcdf/latest/tpt_io_netcdf/parser/struct.NetCdfReader.html

## License

Licensed under either of MIT or Apache-2.0 at your option.
