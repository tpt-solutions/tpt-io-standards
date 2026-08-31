# Changelog

## [0.1.0] - 2026-08-31

Initial release.

- Core parsing implementation
- GeoJSON (RFC 7946) parsing and serialization
- Shapefile (`.shp` / `.shx` / `.dbf`) parsing
- GeoTIFF metadata extraction
- CRS handling (PROJ strings, EPSG registry)
- Datum transformation (Helmert/Bursa–Wolf)
- Reprojection (forward/inverse Transverse Mercator)
- Spatial indexing (R-tree bulk-load, range queries, nearest-neighbour)
- Streaming parser support
- Validation support
- Round-trip serialization
- Fuzz targets
- Golden-file corpus
- Property-based tests
- `no_std` + `alloc` + `std` support
