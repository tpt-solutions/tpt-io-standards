# Changelog

## [0.1.0] - 2026-08-31

Initial release.

- Core parsing implementation
- SBML Level 2 / Level 3 document parsing
- Compartments, species, parameters, and reactions
- Rules (assignment, rate, algebraic), events, and function definitions
- MathML content parsing (infix rendering, reference collection)
- FBC package (flux balance analysis, objectives, bounds, GPR)
- Qual package (qualitative species, transitions)
- Spatial package (geometry, coordinate systems, domains)
- Structural and semantic validation (charge balance, element balance, reference integrity)
- Fuzz targets
- Golden-file corpus
- Property-based tests
- `no_std` + `alloc` + `std` support
