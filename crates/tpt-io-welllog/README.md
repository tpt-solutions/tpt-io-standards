# tpt-io-welllog

Well-log parser for the `tpt-io` workspace — **LAS 2.0 / 3.0** (ASCII **and**
binary) and **DLIS** (RP66 V1, frame-based). All parsers are implemented from
scratch in pure Rust with no C/C++ FFI.

## Features

- **LAS 2.0 / 3.0 ASCII** (`parser`): full section-based grammar
  (`~V` / `~W` / `~O` / `~C` / `~A`), curve-definition extraction
  (mnemonic / unit / description), depth-indexed data extraction, well-header
  extraction (UWI/API, location, dates), `WRAP=YES` multi-line accumulation, and
  a `Reader`/`Writer` pair for round-tripping plus syntactic/semantic validation.
- **LAS 3.0 binary** (`binary`): the `~A` section may carry raw binary records;
  curve data types (`F4`, `F8`, `I1`/`I2`/`I4`, `U1`/`U2`/`U4`, fixed `S<n>`
  strings) declared in the `~C` API-code column are honoured. Automatic
  text-encoding detection (UTF-8 / UTF-16 LE / UTF-16 BE, with UTF-16
  transcoding) means the header is always parseable.
- **DLIS** (`dlis`): RP66 V1 storage-unit-label → visible-record → logical-
  record container parsing; Set-object decoding (`FRAME`, `CHANNEL`, `WELL`,
  `WELLBORE`, `TOOL`, file header) via the standard attribute representation;
  and frame-data reading for the common explicit/implicit formats
  (`FSINGL`, `ISINGL`, `FDOUBL`, `IDOUBL`, `ISLONG`, `ISHORT`).
- **Depth alignment** (`align`): common-grid construction, linear resampling of
  multiple runs, and depth-shift cross-correlation for multi-run correlation.
- **Integration stubs**: the `eng-geotech-integration` feature exposes a
  conversion boundary into the (not-yet-published) `tpt-eng-geotech` crate.

## Example

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

## Testing

The crate ships unit tests plus integration tests (`tests/`): round-trip,
property-based (`proptest`), and golden-file (`tests/samples/`). Fuzz targets
live under `fuzz/` (run with `cargo +nightly fuzz run welllog`).

## Status

LAS 2.0/3.0 ASCII + binary and DLIS structural/frame parsing, depth-alignment
utilities, and the `eng-geotech` integration stub are implemented, with
round-trip, fuzz, golden-file, and property tests. `cargo publish --dry-run` is
blocked only until `tpt-io-core` is published to crates.io (see the workspace
`todo.md` §4 publish order).
