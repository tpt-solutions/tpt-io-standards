# tpt-io-citygml

CityGML 2.0 / 3.0 XML parser for urban and regional 3D models.

## Features

- Parse CityGML 2.0 and 3.0 XML documents into a structured [`CityModel`]
- Extract LOD 0–4 geometry for buildings, vegetation, water bodies, and transportation
- Semantic surface classification (wall, roof, ground, window, door)
- Thematic attribute extraction (function, year built, energy performance, roof type, height)
- Coordinate reference system (CRS) handling with EPSG code extraction
- CityJSON 1.1 serialization support
- Syntactic and semantic validation
- `no_std` compatible (with `alloc`); full `std` by default

## Usage

```toml
[dependencies]
tpt-io-citygml = "0.1"
```

```rust
use tpt_io_core::Reader;
use tpt_io_citygml::parser::CityGmlReader;
use tpt_io_citygml::model::CityModel;

let reader = CityGmlReader::new();
let model: CityModel = reader.parse(xml_bytes).unwrap();
```

## Feature Flags

| Feature | Description |
|---|---|
| `std` (default) | Standard library support |
| `alloc` | Allocation support without `std` |
| `cityjson` | CityJSON 1.1 serialization |
| `geo-integration` | CRS transformation utilities (via `tpt-io-geo`) |
| `eng-geometry-integration` | Integration stub for future `tpt-eng-geometry` |

## License

MIT OR Apache-2.0
