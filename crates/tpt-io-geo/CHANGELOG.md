# Changelog

## Unreleased

- GeoPackage reading: `layers` lists the feature tables, and `read_features` reads one table into a
  feature collection (geometry from the GeoPackage binary header and WKB, attributes as properties, rowid as id).
- `sqlite` module: a read-only SQLite 3 reader (table B-trees, overflow pages, records), which GeoPackage uses.
- WKT, WKB, KML and KMZ (behind the `kmz` feature), and GPX readers and writers.
- FlatGeobuf reading (`flatgeobuf::read`): the FlatBuffers header and columns, the optional packed index, and features
  with typed attributes. Writing is not implemented.
- WKT and WKB readers and writers for the seven simple-feature types, with Z coordinates and EMPTY bodies.
  WKB reads both byte orders and the PostGIS EWKB Z and SRID flags.
- KML 2.2 placemark reader and writer, and KMZ (`doc.kml` in a ZIP) reading and writing behind the `kmz` feature.
- GPX 1.1 reader and writer: waypoints, routes and tracks.

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
