# tpt-io-citygml

CityGML 2.0 / 3.0 XML parser for urban and regional 3D models.

## Quick start

```rust
use tpt_io_core::Reader;
use tpt_io_citygml::parser::CityGmlReader;
use tpt_io_citygml::model::CityModel;

let reader = CityGmlReader::new();
let model: CityModel = reader.parse(xml_bytes).unwrap();
```

## Features

| Feature | Default | Effect |
| --- | --- | --- |
| `std` | yes | Standard library support |
| `alloc` | no | Allocation support without `std` |
| `cityjson` | no | CityJSON 1.1 serialization |
| `geo-integration` | no | CRS transformation utilities (via `tpt-io-geo`) |
| `eng-geometry-integration` | no | Integration stub for future `tpt-eng-geometry` |

## Testing

```sh
cargo test -p tpt-io-citygml
cargo +nightly fuzz run citygml
```

## Status

Per the workspace `todo.md`, `tpt-io-citygml` implements CityGML 2.0 and 3.0
XML parsing, LOD 0–4 geometry extraction, semantic surface classification,
thematic attribute extraction, CRS handling with EPSG code extraction,
CityJSON 1.1 serialization, and syntactic/semantic validation. Round-trip,
fuzz, golden-file, and property-based tests are included. `cargo publish
--dry-run` is blocked until `tpt-io-core` is on crates.io (see §4 publish order).

## License

MIT OR Apache-2.0
