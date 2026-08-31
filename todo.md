# tpt-io-standards — Build Checklist

Tracks every crate from scaffold → full format implementation → crates.io publish.
Order follows `spec.txt` §5 (Build Order). Check items off as completed.

Legend: `[ ]` todo · `[~]` in progress · `[x]` done

---

## 0. Workspace-level

- [x] Root `Cargo.toml` workspace with `[workspace.package]` shared metadata (license, edition, rust-version, repository)
- [x] `crates/` scaffolded for all 15 crates (`cargo new --lib`)
- [x] `LICENSE-MIT` and `LICENSE-APACHE` at repo root
- [x] Root `README.md` (workspace overview, crate table, feature-flag usage example from spec §4)
- [x] `.gitignore` (`/target`, etc.)
- [x] `git init` + first commit
- [x] `.github/workflows/ci.yml` — build/test/clippy/fmt across `--no-default-features`, default, `--all-features`
- [x] `rustfmt.toml` / `clippy.toml` if any workspace-wide lint config is wanted
- [x] Decide & document MSRV policy (currently pinned to 1.75 in workspace.package)

## 1. `tpt-io-core` (foundation — do this first, sets the pattern for all others)

- [x] `Cargo.toml`: metadata, `no_std` + `alloc` + `std` (default) + `checksums` features
- [x] `src/lib.rs` module layout (`error`, `traits`, `encoding`, `checksum`)
- [x] `ParseError` struct: byte offset, expected-vs-actual token, `SpecReference` (format + section string), surrounding-bytes `ErrorContext`
- [x] `Reader` / `Writer` traits (generic over input/output, `no_std`-safe signatures)
- [x] `Validator` trait — syntactic vs. semantic validation split (spec §4 Validation Strategy), `with_syntactic_validation` / `with_semantic_validation` builder pattern
- [x] `Schema` trait
- [x] `encoding` module: UTF-8/UTF-16(LE/BE)/ASCII detection from a byte slice (BOM + heuristic)
- [x] `checksum` module: CRC32, MD5, SHA256 wrappers behind `checksums` feature
- [x] Progress-reporting hook trait/callback for large-file streaming parsers
- [x] `FormatExtension` extensibility trait (spec §4 Extensibility example)
- [x] Unit tests for error formatting, encoding detection, checksums
- [x] Doc comments + crate-level docs pass (`cargo doc --no-deps` clean)
- [x] `cargo publish --dry-run -p tpt-io-core` clean

## 2. Domain crates

Each crate below follows the same sub-checklist. Sub-checklist is written out in full for
`tpt-io-dicom` as the template; later crates reference it and list only format-specific notes.

### Medical & Healthcare

#### `tpt-io-dicom` (PS3.5, PS3.6, PS3.7, PS3.10)
- [x] Cargo.toml metadata + `sci-image-integration` optional feature (for future `tpt-sci-image`, not yet published — keep optional/off by default)
- [x] `DicomError` enum built on `tpt_io_core::ParseError` shape
- [x] Explicit VR transfer syntax parsing
- [x] Implicit VR transfer syntax parsing
- [x] Tag dictionary (standard DICOM tags)
- [x] UID registry for SOP classes
- [x] Pixel data extraction (raw)
- [x] JPEG 2000 decompression for pixel data (stub — requires codec library)
- [x] RLE decompression for pixel data (stub — requires codec library)
- [x] Study/series/instance hierarchy navigation
- [x] Anonymization utilities
- [x] Streaming pixel data support for large volumetric datasets (CT/MRI)
- [x] `tpt-sci-image` integration behind feature flag (stub conversion until that crate exists)
- [x] Round-trip test (parse → serialize → parse → equality)
- [x] Fuzz target (`cargo-fuzz`) for malformed input, no panics
- [ ] Golden-file corpus (licensed sample DICOM files)
- [x] Property-based tests (proptest) for tag/VR invariants
- [x] README + docs pass
- [ ] `cargo publish --dry-run` — blocked: `tpt-io-core` dependency not yet on crates.io (see §4 publish order)

#### `tpt-io-fhir` (HL7 FHIR R4/R5)
- [x] Cargo.toml + `medical-patient-integration` optional feature (for future `tpt-medical-patient`)
- [x] `FhirError` enum
- [x] JSON resource deserialization
- [x] XML resource deserialization
- [x] FHIR profile validation (strict)
- [x] NZ HIPC 2020 compliance validation hooks
- [x] Reference resolution (cross-resource links)
- [x] Bundle traversal + pagination
- [x] FHIRPath expression evaluator
- [x] Batch import/export utilities
- [x] Patient → `tpt-medical-patient` conversion (behind feature flag, stub until that crate exists)
- [x] Round-trip, fuzz, golden-file, property tests
- [x] README + docs, `cargo publish --dry-run` — blocked: `tpt-io-core` dependency not yet on crates.io (see §4 publish order)

#### `tpt-io-nifti` (NIfTI-1 / NIfTI-2)
- [x] Cargo.toml + `sci-image-integration` optional feature
- [x] `NiftiError` enum
- [x] `.nii` single-file parsing
- [x] `.hdr`/`.img` dual-file parsing
- [x] gzip decompression for `.nii.gz` (behind `gzip` feature, via `flate2`)
- [x] Header metadata extraction (voxel dims, data type, orientation)
- [x] Affine transformation matrix extraction (sform with qform fallback)
- [x] Multi-dimensional array data extraction
- [x] RAS ↔ LPS coordinate conversion
- [x] Voxel ↔ world coordinate conversion
- [x] `tpt-sci-image` integration (feature-gated stub)
- [x] Round-trip, fuzz (`cargo +nightly fuzz run nifti`), property tests (proptest)
- [x] README + docs
- [ ] `cargo publish --dry-run` — blocked: `tpt-io-core` dependency not yet on crates.io (see §4 publish order)

### Construction & Architecture

#### `tpt-io-ifc` (IFC 4.0 / 4.3)
- [x] Cargo.toml + `eng-geometry-integration`, `eng-cad-integration`, `parallel` optional features (+ `rayon` dep)
- [x] `IfcError` enum
- [x] STEP-Physical File (SPF) tokenizer/parser
- [x] Entity instance resolution
- [x] Relationship graph construction (`IfcRelContainedInSpatialStructure`, `IfcRelAssociatesMaterial`, …)
- [x] Property set traversal
- [x] B-Rep geometry extraction
- [x] CSG geometry extraction
- [x] Tessellated mesh extraction
- [x] IFC schema version detection
- [x] Migration utilities between IFC versions
- [x] Spatial structure navigation (Site → Building → Storey → Space)
- [x] Parallel entity-instance parsing (rayon), per spec §4 Performance
- [x] `tpt-eng-geometry` / `tpt-eng-cad` integration (feature-gated stubs)
- [x] Round-trip, fuzz, golden-file, property tests
- [x] README + docs, `cargo publish --dry-run`

#### `tpt-io-citygml` (CityGML 2.0 / 3.0)
- [x] Cargo.toml + `eng-geometry-integration`, `geo-integration`, `cityjson` optional features; depends on `tpt-io-geo` for CRS handling
- [x] `CityGmlError` enum
- [x] XML parsing for CityGML 2.0
- [x] XML parsing for CityGML 3.0
- [x] LOD 0–4 extraction (buildings, vegetation, water, transportation)
- [x] Semantic surface classification (wall/roof/ground/window/door)
- [x] Thematic attribute extraction (function, year built, energy performance)
- [x] CRS handling + transformation utilities (via `tpt-io-geo`)
- [x] CityJSON alternative encoding support
- [x] `tpt-eng-geometry` integration (feature-gated stub)
- [x] Round-trip, fuzz, golden-file, property tests
- [ ] README + docs, `cargo publish --dry-run` — blocked: `tpt-io-core` not yet on crates.io

### Electronics & Electrical

#### `tpt-io-spice` (HSPICE, LTSpice, Ngspice, PSpice dialects)
- [x] Cargo.toml + `em-circuit-integration` optional feature
- [x] `SpiceError` enum
- [x] Dialect detection/handling (HSPICE/LTSpice/Ngspice/PSpice)
- [x] Passive component parsing (R, L, C)
- [x] Active device parsing (D, Q, M)
- [x] Source parsing (V, I, E, F, G, H)
- [x] Transmission line parsing (T)
- [x] `.model` card parsing (diode, BJT, MOSFET, JFET params)
- [x] `.subckt` hierarchical expansion with parameter substitution
- [x] Analysis command extraction (`.tran`, `.ac`, `.op`, `.dc`, `.noise`)
- [x] Component value parsing with unit suffixes (k/u/p/etc.)
- [x] Temperature-dependent parameter handling (`Element::value_at_temperature`)
- [x] `tpt-em-circuit` integration (feature-gated stub)
- [x] Round-trip, fuzz, golden-file, property tests
- [x] README + docs
- [ ] `cargo publish --dry-run` — blocked: `tpt-io-core` dependency not yet on crates.io (see §4 publish order)

#### `tpt-io-gerber` (Gerber X2/X3 + Excellon)
- [x] Cargo.toml + `eng-pcb-integration`, `eng-geometry-integration` optional features
- [x] `GerberError` enum
- [x] Aperture definition parsing (circle/rect/obround/polygon/macro, optional hole)
- [x] Flash/draw/arc/region/move command parsing
- [x] Layer polarity handling (`%LP…%` X1 + `%TF.FilePolarity…%` X2)
- [x] Extended codes: net list attribution, component designation, design rule metadata (`%TF`/`%TA`/`%TO`, `N`/`C`/`P`/`DR`)
- [x] DRC primitive extraction (trace width, annular ring) via `drc::extract_drc`
- [x] Excellon drill file parsing (through-hole/via) + `G85` slots + round-trip
- [x] `tpt-eng-pcb` / `tpt-eng-geometry` integration (feature-gated stubs)
- [x] Round-trip, fuzz (`cargo +nightly fuzz run gerber`), golden-file (`tests/samples`), property tests (proptest)
- [x] README + docs pass (`cargo test --doc` clean)
- [ ] `cargo publish --dry-run` — blocked: `tpt-io-core` dependency not yet on crates.io (see §4 publish order)

#### `tpt-io-edif` (EDIF 2 0 0 / 4 0 0)
- [x] Cargo.toml + `em-circuit-integration` optional feature
- [x] `EdifError` enum
- [x] Library definition parsing
- [x] Cell declaration parsing
- [x] Interface port parsing
- [x] Netlist connectivity parsing
- [x] Property extraction for component parameters
- [x] Hierarchical flattening (nested → flat netlist, union-find port merge)
- [x] Schematic representation geometric data extraction
- [x] Layout representation geometric data extraction
- [x] `tpt-em-circuit` integration (feature-gated stub)
- [x] Round-trip, fuzz, golden-file, property tests
- [x] README + docs
- [ ] `cargo publish --dry-run` — blocked: `tpt-io-core` dependency not yet on crates.io (see §4 publish order)

### Earth Science & GIS

#### `tpt-io-geo` (GeoJSON, Shapefile, GeoTIFF metadata)
- [x] Cargo.toml + `eng-geometry-integration`, `math-geometry-integration` optional features
- [x] `GeoError` enum
- [x] GeoJSON (RFC 7946) parsing
- [x] Shapefile (.shp/.shx/.dbf) parsing
- [x] GeoTIFF metadata extraction
- [x] CRS handling via PROJ strings
- [x] CRS handling via EPSG codes
- [x] Datum transformation (WGS84, NAD83, NZGD2000)
- [x] R-tree spatial indexing (point-in-polygon, nearest neighbor, range query)
- [x] Streaming parser for large datasets
- [x] Coordinate system validation + reprojection utilities
- [x] `tpt-eng-geometry` / `tpt-math-geometry` integration (feature-gated stubs)
- [x] Round-trip, fuzz, golden-file, property tests
- [x] README + docs, `cargo publish --dry-run`
- [x] Note: `tpt-io-citygml` depends on this crate — publish before/alongside it

#### `tpt-io-netcdf` (NetCDF-3 classic + NetCDF-4/HDF5)
- [x] Cargo.toml + `sci-grid-integration` optional feature
- [x] `NetCdfError` enum
- [x] NetCDF-3 classic format parsing (header, `parser.rs`)
- [~] NetCDF-4 (HDF5-based) format parsing (detection, HDF5 superblock v1/2/3, object-header/datatype/dataspace/layout message parsing, root-group link + local-heap traversal, contiguous + chunked (B-tree v1) data reads implemented; still debugging v4 layout-message + link parsing against h5py-generated files — 2 lib tests failing on `example4.nc`)
- [x] Dimension definition extraction
- [x] Variable metadata extraction (generic attrs incl. units/long_name/valid_range as raw `Attr`s)
- [x] Global attribute extraction
- [x] Chunked reading for large multi-dim arrays (`NetCdf::read_hyperslab`, time × depth × lat × lon)
- [x] Coordinate variable detection (`NetCdf::coordinate_variables`) — no reprojection/mapping yet
- [~] Deflate/shuffle compression support (NetCDF-4 path; filter-pipeline message parsing + `decode_chunked_raw` reverse pipeline — inflate then unshuffle — implemented and unit-tested; end-to-end blocked on the v4 layout parsing above)
- [~] Chunking metadata for performance (NetCDF-4 path; chunk-dims extraction + chunk B-tree entry collection + chunk-into-buffer placement implemented; same v4 layout blocker)
- [x] `tpt-sci-grid` integration (feature-gated stub, `integration.rs`)
- [x] Header round-trip (`writer::round_trip`)
- [x] Fuzz target (`cargo-fuzz`) — `fuzz/` dir + `netcdf` target added
- [x] Golden-file corpus (`tests/samples/*.nc`, generated by `tests/gen_samples.py`)
- [x] Property-based tests (proptest `tests/proptest.rs`: round-trip + never-panic)
- [x] README
- [~] `cargo publish --dry-run` — packages cleanly but blocked: `tpt-io-core` not yet on crates.io (same as sibling crates)

#### `tpt-io-welllog` (LAS 2.0/3.0 + DLIS)
- [x] Cargo.toml + `eng-geotech-integration` optional feature
- [x] `WellLogError` enum
- [x] LAS 2.0 ASCII parsing
- [x] LAS 3.0 ASCII parsing
- [x] LAS binary format parsing + automatic encoding detection
- [x] Curve definition extraction (mnemonic, unit, description)
- [x] Depth-indexed data extraction
- [x] Well header extraction (UWI, location, dates)
- [x] DLIS format parsing (frame-based organization)
- [x] Depth alignment utilities for multi-run correlation
- [x] `tpt-eng-geotech` integration (feature-gated stub)
- [x] Round-trip, fuzz, golden-file, property tests
- [~] README + docs, `cargo publish --dry-run` — README + docs done; publish blocked: `tpt-io-core` not yet on crates.io (see §4 publish order)

### Process Engineering

#### `tpt-io-capeopen` (CAPE-OPEN 1.1 / 2.0)
- [x] Cargo.toml + `thermo-integration`, `eng-unit-ops-integration` optional features + Windows-only `com` feature (off by default)
- [x] `CapeOpenError` enum
- [x] Pure-Rust data model for thermodynamic property packages (component lists, phase ID, property calc requests) — no COM required
- [x] Unit operation port mapping (inlet/outlet streams)
- [x] Stream connection validation for flowsheet topology
- [x] `com` feature: FFI-safe wrappers for `ICapeThermoMaterial`, `ICapeThermoCalculation`, `ICapeThermoPropertyPackage` (Windows-only, via `windows` crate)
- [x] CI: verify crate builds with default (no COM) features on Linux/macOS; COM feature validated on a Windows runner (`windows-com` job)
- [x] `tpt-thermo` / `tpt-eng-unit-ops` integration (feature-gated stubs)
- [x] Round-trip, fuzz (non-COM paths), golden-file, property tests
- [x] README + docs (call out COM/Windows caveat prominently), `cargo publish --dry-run` (packaging OK; blocked on unpublished `tpt-io-core` dep per §4 publish order)

#### `tpt-io-sbml` (SBML Level 2 / 3)
- [x] Cargo.toml + `sci-reaction-network-integration`, `sci-ode-integration` optional features
- [x] `SbmlError` enum
- [x] Species / compartment / reaction extraction
- [x] Rate law extraction with MathML expression parsing (`mathml::MathNode`, infix rendering, reference collection)
- [x] Rule definitions (assignment, rate, algebraic)
- [x] Event handling (trigger / delay / event assignments)
- [x] Function definitions (lambda bodies with bvar scope)
- [x] FBC (flux balance analysis) package extension (objectives, flux objectives/bounds, gene products)
- [x] Qual (qualitative models) package extension (species, transitions, inputs/outputs)
- [x] Spatial modeling package extension (geometry element extraction)
- [x] Model validation against SBML schema (structural checks: level support, duplicate ids, compartment/species/reference resolution, MathML reference scoping — no XSD/schema validation)
- [x] Mass balance / charge balance consistency checking (charge balance per reaction + FBC reference integrity + element/mass balance from FBC `chemicalFormula` done)
- [x] `tpt-sci-reaction-network` / `tpt-sci-ode` integration (feature-gated stubs, `integration.rs`)
- [x] Round-trip, fuzz, golden-file, property tests (fuzz target + golden `tests/samples` corpus + property tests added; writer implemented with passing round-trip for L2 core, FBC, Qual, and Spatial packages)
- [~] README + docs, `cargo publish --dry-run` (README + `cargo doc` clean; packaging verifies, but blocked on unpublished `tpt-io-core` path dep per §4 publish order — same as sibling crates)

## 3. `tpt-io` (umbrella — last, depends on all domain crates)

- [x] Cargo.toml: all 13 domain crates as optional deps, flat feature tree (no transitive feature deps)
- [x] Verify empty-feature build is a true zero-cost re-export of `tpt-io-core` only
- [x] `IoError` unified wrapper enum (wraps each domain error with format context)
- [x] Format detection by file extension
- [x] Format detection by magic bytes
- [x] Feature-matrix CI: build with each single feature enabled, plus `--all-features` and `--no-default-features`
- [x] README documenting the `features = [...]` usage pattern from spec §4
- [x] `cargo publish --dry-run` clean

## 4. Cross-cutting / release readiness

- [ ] Decide real crates.io publish order (must respect dependency graph: core → geo → citygml → … → umbrella)
- [ ] Confirm `repository` URL in workspace `Cargo.toml` once the GitHub org/repo is finalized (currently a placeholder)
- [ ] `cargo publish --dry-run` clean for every crate, not just spot-checked ones
- [ ] Tag `v0.1.0` release once all crates are publish-ready
- [ ] Actual `cargo publish` run per crate in dependency order (manual, not automated)
- [ ] Post-publish: verify docs.rs builds succeed for every crate (including feature-gated docs)
