# tpt-io-welllog

Well-log parser for the `tpt-io` workspace — **LAS 2.0 / 3.0** (ASCII **and**
binary) and **DLIS** (RP66 V1, frame-based). All parsers are implemented from
scratch in pure Rust with no C/C++ FFI.

## Quick start

```rust
use tpt_io_core::Reader;
use tpt_io_welllog::parser::LasReader;

let src = b"~Version Information\nVERS. 2.0 :\nWRAP. NO :\n\
~Well Information\nWELL. DEMO :\n~Curve Information\nDEPT. M :\nGR . API :\n\
~ASCII Log Data\n10.0 20.0\n20.0 21.0\n";
let log = LasReader::new().parse(src).unwrap();
assert_eq!(log.curves.len(), 2);
assert_eq!(log.index_values().collect::<Vec<_>>(), vec![10.0, 20.0]);
```

DLIS:

```rust
use tpt_io_welllog::dlis;

let dlis = dlis::parse(&bytes)?;
for frame in &dlis.frames {
    println!("frame {}: {} samples", frame.name, frame.samples.len());
}
```

## Features

| Feature | Default | Effect |
| --- | --- | --- |
| `std` | yes | `std`-only conveniences |
| `alloc` | no | `alloc`-based types without `std` |
| `eng-geotech-integration` | no | Stub conversion into a future `tpt-eng-geotech` crate |

## Testing

The crate ships unit tests plus integration tests (`tests/`): round-trip,
property-based (`proptest`), and golden-file (`tests/samples/`). Fuzz targets
live under `fuzz/` (run with `cargo +nightly fuzz run welllog`).

```sh
cargo test -p tpt-io-welllog
cargo +nightly fuzz run welllog
```

## Status

LAS 2.0/3.0 ASCII + binary and DLIS structural/frame parsing, depth-alignment
utilities, and the `eng-geotech` integration stub are implemented, with
round-trip, fuzz, golden-file, and property tests. `cargo publish --dry-run` is
blocked only until `tpt-io-core` is published to crates.io (see the workspace
`todo.md` §4 publish order).

## License

Licensed under either of [MIT](../LICENSE-MIT) or [Apache-2.0](../LICENSE-APACHE)
at your option.
