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
- [ ] Explicit VR transfer syntax parsing
- [ ] Implicit VR transfer syntax parsing
- [ ] Tag dictionary (standard DICOM tags)
- [ ] UID registry for SOP classes
- [ ] Pixel data extraction (raw)
- [ ] JPEG 2000 decompression for pixel data
- [ ] RLE decompression for pixel data
- [ ] Study/series/instance hierarchy navigation
- [ ] Anonymization utilities
- [ ] Streaming pixel data support for large volumetric datasets (CT/MRI)
- [ ] `tpt-sci-image` integration behind feature flag (stub conversion until that crate exists)
- [ ] Round-trip test (parse → serialize → parse → equality)
- [ ] Fuzz target (`cargo-fuzz`) for malformed input, no panics
- [ ] Golden-file corpus (licensed sample DICOM files)
- [ ] Property-based tests (proptest) for tag/VR invariants
- [ ] README + docs pass
- [ ] `cargo publish --dry-run` clean

#### `tpt-io-fhir` (HL7 FHIR R4/R5)
- [ ] Cargo.toml + `medical-patient-integration` optional feature (for future `tpt-medical-patient`)
- [x] `FhirError` enum
- [ ] JSON resource deserialization
- [ ] XML resource deserialization
- [ ] FHIR profile validation (strict)
- [ ] NZ HIPC 2020 compliance validation hooks
- [ ] Reference resolution (cross-resource links)
- [ ] Bundle traversal + pagination
- [ ] FHIRPath expression evaluator
- [ ] Batch import/export utilities
- [ ] Patient → `tpt-medical-patient` conversion (behind feature flag, stub until that crate exists)
- [ ] Round-trip, fuzz, golden-file, property tests
- [ ] README + docs, `cargo publish --dry-run`

#### `tpt-io-nifti` (NIfTI-1 / NIfTI-2)
- [ ] Cargo.toml + `sci-image-integration` optional feature
- [x] `NiftiError` enum
- [ ] `.nii` single-file parsing
- [ ] `.hdr`/`.img` dual-file parsing
- [ ] gzip decompression for `.nii.gz`
- [ ] Header metadata extraction (voxel dims, data type, orientation)
- [ ] Affine transformation matrix extraction
- [ ] Multi-dimensional array data extraction
- [ ] RAS ↔ LPS coordinate conversion
- [ ] Voxel ↔ world coordinate conversion
- [ ] `tpt-sci-image` integration (feature-gated stub)
- [ ] Round-trip, fuzz, golden-file, property tests
- [ ] README + docs, `cargo publish --dry-run`

### Construction & Architecture

#### `tpt-io-ifc` (IFC 4.0 / 4.3)
- [ ] Cargo.toml + `eng-geometry-integration`, `eng-cad-integration` optional features
- [x] `IfcError` enum
- [ ] STEP-Physical File (SPF) tokenizer/parser
- [ ] Entity instance resolution
- [ ] Relationship graph construction (`IfcRelContainedInSpatialStructure`, `IfcRelAssociatesMaterial`, …)
- [ ] Property set traversal
- [ ] B-Rep geometry extraction
- [ ] CSG geometry extraction
- [ ] Tessellated mesh extraction
- [ ] IFC schema version detection
- [ ] Migration utilities between IFC versions
- [ ] Spatial structure navigation (Site → Building → Storey → Space)
- [ ] Parallel entity-instance parsing (rayon), per spec §4 Performance
- [ ] `tpt-eng-geometry` / `tpt-eng-cad` integration (feature-gated stubs)
- [ ] Round-trip, fuzz, golden-file, property tests
- [ ] README + docs, `cargo publish --dry-run`

#### `tpt-io-citygml` (CityGML 2.0 / 3.0)
- [ ] Cargo.toml + `eng-geometry-integration` optional feature; depends on `tpt-io-geo` for CRS handling
- [x] `CityGmlError` enum
- [ ] XML parsing for CityGML 2.0
- [ ] XML parsing for CityGML 3.0
- [ ] LOD 0–4 extraction (buildings, vegetation, water, transportation)
- [ ] Semantic surface classification (wall/roof/ground/window/door)
- [ ] Thematic attribute extraction (function, year built, energy performance)
- [ ] CRS handling + transformation utilities (via `tpt-io-geo`)
- [ ] CityJSON alternative encoding support
- [ ] `tpt-eng-geometry` integration (feature-gated stub)
- [ ] Round-trip, fuzz, golden-file, property tests
- [ ] README + docs, `cargo publish --dry-run`

### Electronics & Electrical

#### `tpt-io-spice` (HSPICE, LTSpice, Ngspice, PSpice dialects)
- [ ] Cargo.toml + `em-circuit-integration` optional feature
- [x] `SpiceError` enum
- [x] Dialect detection/handling (HSPICE/LTSpice/Ngspice/PSpice)
- [x] Passive component parsing (R, L, C)
- [x] Active device parsing (D, Q, M)
- [x] Source parsing (V, I, E, F, G, H)
- [x] Transmission line parsing (T)
- [x] `.model` card parsing (diode, BJT, MOSFET, JFET params)
- [ ] `.subckt` hierarchical expansion with parameter substitution
- [x] Analysis command extraction (`.tran`, `.ac`, `.op`, `.dc`, `.noise`)
- [x] Component value parsing with unit suffixes (k/u/p/etc.)
- [ ] Temperature-dependent parameter handling
- [ ] `tpt-em-circuit` integration (feature-gated stub)
- [ ] Round-trip, fuzz, golden-file, property tests
- [ ] README + docs, `cargo publish --dry-run`

#### `tpt-io-gerber` (Gerber X2/X3 + Excellon)
- [ ] Cargo.toml + `eng-pcb-integration`, `eng-geometry-integration` optional features
- [x] `GerberError` enum
- [ ] Aperture definition parsing
- [ ] Flash/draw command parsing
- [ ] Layer polarity handling
- [ ] Extended codes: net list attribution, component designation, design rule metadata
- [ ] DRC primitive extraction (trace width, spacing, annular ring)
- [ ] Excellon drill file parsing (through-hole/via)
- [ ] `tpt-eng-pcb` / `tpt-eng-geometry` integration (feature-gated stubs)
- [ ] Round-trip, fuzz, golden-file, property tests
- [ ] README + docs, `cargo publish --dry-run`

#### `tpt-io-edif` (EDIF 2 0 0 / 4 0 0)
- [ ] Cargo.toml + `em-circuit-integration` optional feature
- [x] `EdifError` enum
- [ ] Library definition parsing
- [ ] Cell declaration parsing
- [ ] Interface port parsing
- [ ] Netlist connectivity parsing
- [ ] Property extraction for component parameters
- [ ] Hierarchical flattening (nested → flat netlist)
- [ ] Schematic representation geometric data extraction
- [ ] Layout representation geometric data extraction
- [ ] `tpt-em-circuit` integration (feature-gated stub)
- [ ] Round-trip, fuzz, golden-file, property tests
- [ ] README + docs, `cargo publish --dry-run`

### Earth Science & GIS

#### `tpt-io-geo` (GeoJSON, Shapefile, GeoTIFF metadata)
- [ ] Cargo.toml + `eng-geometry-integration`, `math-geometry-integration` optional features
- [x] `GeoError` enum
- [ ] GeoJSON (RFC 7946) parsing
- [ ] Shapefile (.shp/.shx/.dbf) parsing
- [ ] GeoTIFF metadata extraction
- [ ] CRS handling via PROJ strings
- [ ] CRS handling via EPSG codes
- [ ] Datum transformation (WGS84, NAD83, NZGD2000)
- [ ] R-tree spatial indexing (point-in-polygon, nearest neighbor, range query)
- [ ] Streaming parser for large datasets
- [ ] Coordinate system validation + reprojection utilities
- [ ] `tpt-eng-geometry` / `tpt-math-geometry` integration (feature-gated stubs)
- [ ] Round-trip, fuzz, golden-file, property tests
- [ ] README + docs, `cargo publish --dry-run`
- [ ] Note: `tpt-io-citygml` depends on this crate — publish before/alongside it

#### `tpt-io-netcdf` (NetCDF-3 classic + NetCDF-4/HDF5)
- [ ] Cargo.toml + `sci-grid-integration` optional feature
- [x] `NetCdfError` enum
- [ ] NetCDF-3 classic format parsing
- [ ] NetCDF-4 (HDF5-based) format parsing
- [ ] Dimension definition extraction
- [ ] Variable metadata extraction (units, long_name, valid_range)
- [ ] Global attribute extraction
- [ ] Chunked reading for large multi-dim arrays (time × depth × lat × lon)
- [ ] Coordinate variable detection + automatic coordinate mapping
- [ ] Deflate/shuffle compression support
- [ ] Chunking metadata for performance
- [ ] `tpt-sci-grid` integration (feature-gated stub)
- [ ] Round-trip, fuzz, golden-file, property tests
- [ ] README + docs, `cargo publish --dry-run`

#### `tpt-io-welllog` (LAS 2.0/3.0 + DLIS)
- [ ] Cargo.toml + `eng-geotech-integration` optional feature
- [x] `WellLogError` enum
- [x] LAS 2.0 ASCII parsing
- [x] LAS 3.0 ASCII parsing
- [ ] LAS binary format parsing + automatic encoding detection
- [x] Curve definition extraction (mnemonic, unit, description)
- [x] Depth-indexed data extraction
- [x] Well header extraction (UWI, location, dates)
- [ ] DLIS format parsing (frame-based organization)
- [ ] Depth alignment utilities for multi-run correlation
- [ ] `tpt-eng-geotech` integration (feature-gated stub)
- [ ] Round-trip, fuzz, golden-file, property tests
- [ ] README + docs, `cargo publish --dry-run`

### Process Engineering

#### `tpt-io-capeopen` (CAPE-OPEN 1.1 / 2.0)
- [ ] Cargo.toml + `thermo-integration`, `eng-unit-ops-integration` optional features + Windows-only `com` feature (off by default)
- [x] `CapeOpenError` enum
- [ ] Pure-Rust data model for thermodynamic property packages (component lists, phase ID, property calc requests) — no COM required
- [ ] Unit operation port mapping (inlet/outlet streams)
- [ ] Stream connection validation for flowsheet topology
- [ ] `com` feature: FFI-safe wrappers for `ICapeThermoMaterial`, `ICapeThermoCalculation`, `ICapeThermoPropertyPackage` (Windows-only, via `windows` crate)
- [ ] CI: verify crate builds with default (no COM) features on Linux/macOS; COM feature validated on a Windows runner
- [ ] `tpt-thermo` / `tpt-eng-unit-ops` integration (feature-gated stubs)
- [ ] Round-trip, fuzz (non-COM paths), golden-file, property tests
- [ ] README + docs (call out COM/Windows caveat prominently), `cargo publish --dry-run`

#### `tpt-io-sbml` (SBML Level 2 / 3)
- [ ] Cargo.toml + `sci-reaction-network-integration`, `sci-ode-integration` optional features
- [x] `SbmlError` enum
- [ ] Species / compartment / reaction extraction
- [ ] Rate law extraction with MathML expression parsing
- [ ] Rule definitions (assignment, rate, algebraic)
- [ ] Event handling
- [ ] Function definitions
- [ ] FBC (flux balance analysis) package extension
- [ ] Qual (qualitative models) package extension
- [ ] Spatial modeling package extension
- [ ] Model validation against SBML schema
- [ ] Mass balance / charge balance consistency checking
- [ ] `tpt-sci-reaction-network` / `tpt-sci-ode` integration (feature-gated stubs)
- [ ] Round-trip, fuzz, golden-file, property tests
- [ ] README + docs, `cargo publish --dry-run`

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
