# tpt-io-geo

Geospatial data-format parser for the `tpt-io` workspace — GeoJSON (RFC 7946),
Esri Shapefile (`.shp` / `.shx` / `.dbf`), and GeoTIFF metadata extraction —
plus coordinate-reference-system handling, datum transformation, reprojection,
spatial indexing, and streaming parsers.

All parsers are implemented from scratch in pure Rust with no C/C++ FFI.

## Quick start

```rust
use tpt_io_core::Reader;
use tpt_io_geo::geojson::{GeoJsonReader, FeatureCollection};

let src = br#"{"type":"FeatureCollection","features":[
  {"type":"Feature",
   "geometry":{"type":"Point","coordinates":[174.0,-41.0]},
   "properties":{"name":"Wellington"}}
]}"#;
let fc: FeatureCollection = GeoJsonReader::new().parse(src).unwrap();
assert_eq!(fc.features.len(), 1);
```

```rust
use tpt_io_geo::crs::{Crs, project, reproject};

// Project a lon/lat into its UTM zone and back.
let crs = Crs::from_epsg(32760).unwrap(); // UTM 60S
let (x, y) = project(&crs, 174.0, -41.0).unwrap();
let (lon, lat) = reproject(&crs, &Crs::wgs84(), x, y).unwrap();
assert!((lon - 174.0).abs() < 1e-7);
```

## Features

| Feature | Default | Effect |
| --- | --- | --- |
| `std` | yes | `std`-only conveniences |
| `alloc` | no | `alloc`-based types without `std` |
| `eng-geometry-integration` | no | Stub conversion into a future `tpt-eng-geometry` crate |
| `math-geometry-integration` | no | Stub conversion into a future `tpt-math-geometry` crate |

## Testing

The crate ships unit tests plus integration tests (`tests/`): round-trip,
property-based (`proptest`), and golden-file (`tests/samples/`). Fuzz targets
live under `fuzz/` (run with `cargo +nightly fuzz run <target>`).

```sh
cargo test -p tpt-io-geo
cargo +nightly fuzz run geo
```

## Status

Implemented for the core Earth-science subset described in `spec.txt §3`:
GeoJSON, Shapefile, GeoTIFF metadata, PROJ/EPSG CRS, WGS84/NAD83/NZGD2000 datum
transforms, TM reprojection, R-tree spatial queries, streaming parsers, fuzz,
golden-file, and property tests. `cargo publish --dry-run` is blocked only until
`tpt-io-core` is published to crates.io (see workspace `todo.md` §4 publish order).

## License

Licensed under either of [MIT](../LICENSE-MIT) or [Apache-2.0](../LICENSE-APACHE)
at your option.
