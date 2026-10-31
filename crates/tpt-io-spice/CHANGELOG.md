# Changelog

## Unreleased

- `raw` module: SPICE `.raw` simulation output, read in binary and ASCII form and written in binary, for real-valued
  plots (complex plots are rejected).

## [0.1.0] - 2026-08-31

Initial release.

- Core parsing implementation
- HSPICE / LTSpice / Ngspice / PSpice dialect support
- Passive and active device card parsing
- SI-suffixed value scaling
- `.model`, `.subckt`/`.ends`, and analysis commands
- Temperature-dependent value resolution
- Validation support
- Round-trip serialization
- Fuzz targets
- Golden-file corpus
- Property-based tests
- `no_std` + `alloc` + `std` support
