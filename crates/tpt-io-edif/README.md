# tpt-io-edif

EDIF 2.0.0 / 4.0.0 parser for the `tpt-io` workspace — hierarchical electronic
design interchange format (netlists, libraries, cells, and views).

## Quick start

```rust
use tpt_io_core::Reader;
use tpt_io_edif::parser::EdifReader;
use tpt_io_edif::model::Edif;

let src = b"(edif top (edifVersion 2 0 0) (edifLevel 0) \
  (library lib (cell c (view v (viewType NETLIST) \
    (interface (port a (direction IN)) (port z (direction OUT))) \
    (contents (net n (joined (portRef a) (portRef z))))))) \
  (design d (cellRef c (libraryRef lib)))";
let edif: Edif = EdifReader::new().parse(src).unwrap();
let flat = tpt_io_edif::flatten::flatten(&edif).unwrap();
assert_eq!(flat.nets.len(), 1);
```

## Features

| Feature | Default | Effect |
| --- | --- | --- |
| `std` | yes | `std`-only conveniences |
| `alloc` | no | `alloc`-based types without `std` |
| `em-circuit-integration` | no | Stub conversion into a future `tpt-em-circuit` crate |

## Testing

```sh
cargo test -p tpt-io-edif
cargo +nightly fuzz run edif
```

## Status

Per the workspace `todo.md`, EDIF is implemented for the netlist-relevant subset
(library/cell/view/interface/instance/net extraction, hierarchical flattening,
schematic & layout geometric extraction, round-trip, fuzz, golden-file, and
property tests). `cargo publish --dry-run` is blocked until `tpt-io-core` is on
crates.io (see §4 publish order).

## License

Licensed under either of [MIT](../LICENSE-MIT) or [Apache-2.0](../LICENSE-APACHE)
at your option.
