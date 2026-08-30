# tpt-io-geo

Geospatial data-format parser for the `tpt-io` workspace — GeoJSON (RFC 7946),
Esri Shapefile (`.shp` / `.shx` / `.dbf`), and GeoTIFF metadata extraction —
plus coordinate-reference-system handling, datum transformation, reprojection,
spatial indexing, and streaming parsers.

All parsers are implemented from scratch in pure Rust with no C/C++ FFI.

## Features

- **GeoJSON** (`geojson`): RFC 7946 `Feature` / `FeatureCollection` /
  `Geometry` parsing and serialisation, foreign-member preservation, and
  syntactic/semantic validation (range checks on lon/lat).
- **Shapefile** (`shapefile`): `.shp` geometry (Point, PolyLine, Polygon,
  MultiPoint, and the `Z`/`M` variants), `.shx` index, and `.dbf` (dBASE III+)
  attribute tables, unified into a `ShapefileDataset`.
- **GeoTIFF metadata** (`geotiff`): TIFF container parsing (II/MM byte order)
  plus extraction of the georeferencing tags — `ModelPixelScale`, `ModelTiePoint`,
  `ModelTransformation`, the `GeoKeyDirectory`, and the resulting EPSG code,
  tie points, and ground-space bounding box. Pixel samples are **not** decoded.
- **CRS** (`crs`): parse/validate PROJ `+key=value` strings; a small built-in
  EPSG registry (WGS84, NAD83, NZGD2000, UTM zones, NZTM, …); Helmert
  (Bursa–Wolf) datum transformation between WGS84 / NAD83 / NZGD2000; and
  forward/inverse Transverse Mercator (Redfearn) reprojection.
- **Spatial indexing** (`spatial_index`): a static R-tree (Sort-Tile-Recursive
  bulk load) supporting range queries, nearest-neighbour search, and
  point-in-polygon candidate selection (ray casting via `point_in_ring`).
- **Streaming** (`stream`): `GeoJsonFeatureStream` and `ShapefileRecordStream`
  iterate a single feature/record at a time over a `&[u8]` buffer.
- **Integration stubs**: `eng-geometry-integration` and
  `math-geometry-integration` features expose conversion boundaries into the
  (not-yet-published) `tpt-eng-geometry` / `tpt-math-geometry` crates.

## Example

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

## Testing

The crate ships unit tests plus integration tests (`tests/`): round-trip,
property-based (`proptest`), and golden-file (`tests/samples/`). Fuzz targets
live under `fuzz/` (run with `cargo +nightly fuzz run <target>`).

## Status

Implemented for the core Earth-science subset described in `spec.txt §3`:
GeoJSON, Shapefile, GeoTIFF metadata, PROJ/EPSG CRS, WGS84/NAD83/NZGD2000 datum
transforms, TM reprojection, R-tree spatial queries, streaming parsers, fuzz,
golden-file, and property tests. `cargo publish --dry-run` is blocked only until
`tpt-io-core` is published to crates.io (see workspace `todo.md` §4 publish order).
