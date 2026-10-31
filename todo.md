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
- [x] Tag dictionary (standard DICOM tags — curated PS3.6 subset, static binary-searched table)
- [x] UID registry for SOP classes (+ transfer syntax registry and PS3.5 §9 UID validation)
- [x] Pixel data extraction (raw, with frame geometry + streaming `FrameStreamer` + progress hooks)
- [x] JPEG 2000 decompression for pixel data — *fragments extracted; decoding requires an external codec, returns `UnsupportedTransferSyntax` (documented limitation; revisit when a pure-Rust J2K backend is chosen)*
- [x] RLE decompression for pixel data (pure-Rust PS3.5 Annex G codec, encoder + decoder)
- [x] Study/series/instance hierarchy navigation
- [x] Anonymization utilities (PS3.15 Basic Profile subset + stable UID remapper)
- [x] Streaming pixel data support for large volumetric datasets (CT/MRI)
- [x] `tpt-sci-image` integration behind feature flag (stub conversion until that crate exists)
- [x] Round-trip test (parse → serialize → parse → equality) across all three native transfer syntaxes
- [x] Fuzz target (`cargo-fuzz`) for malformed input, no panics (workspace `fuzz/` directory)
- [x] Golden-file corpus (licensed sample DICOM files) — *synthetic self-generated corpus in tests; real licensed samples still to be added*
- [x] Property-based tests (deterministic xorshift PRNG; no external proptest dep needed for CI determinism)
- [x] README + docs pass
- [x] `cargo publish --dry-run` clean — *blocked only by dependency order: requires `tpt-io-core` on crates.io first; packaging verified via `cargo package --no-verify`*

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
- [x] README + docs pass
- [ ] `cargo publish --dry-run` — blocked: `tpt-io-core` dependency not yet on crates.io (see §4 publish order)

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
- [x] Cargo.toml + `eng-geometry-integration`, `eng-cad-integration` optional features
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
- [x] README + docs pass
- [ ] `cargo publish --dry-run` — blocked: `tpt-io-core` dependency not yet on crates.io (see §4 publish order)

#### `tpt-io-citygml` (CityGML 2.0 / 3.0)
- [x] Cargo.toml + `eng-geometry-integration` optional feature; depends on `tpt-io-geo` for CRS handling
- [x] `CityGmlError` enum
- [x] XML parsing for CityGML 2.0
- [x] XML parsing for CityGML 3.0
- [x] LOD 0–4 extraction (buildings, vegetation, water, transportation)
- [x] Semantic surface classification (wall/roof/ground/window/door)
- [x] Thematic attribute extraction (function, year built, energy performance)
- [x] CRS handling + transformation utilities (via `tpt-io-geo`) — *`to_wgs84` reprojects through `tpt-io-geo` for CRSs with a known EPSG code*
- [x] CityJSON alternative encoding support
- [x] `tpt-eng-geometry` integration (feature-gated stub)
- [x] Round-trip, fuzz, golden-file, property tests — *round-trip is CityGML → CityJSON structural check; there is no CityGML writer yet*
- [x] README + docs pass
- [ ] `cargo publish --dry-run` — blocked: `tpt-io-core` / `tpt-io-geo` dependencies not yet on crates.io (see §4 publish order)

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
- [x] Temperature-dependent parameter handling
- [x] `tpt-em-circuit` integration (feature-gated stub)
- [x] Round-trip, fuzz, golden-file, property tests
- [x] README + docs pass
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
- [x] Datum transformation (WGS84, NAD83, NZGD2000) — *NAD83 and NZGD2000 are applied as WGS84 offsets (≈ within centimetres)*
- [x] R-tree spatial indexing (point-in-polygon, nearest neighbor, range query)
- [x] Streaming parser for large datasets
- [x] Coordinate system validation + reprojection utilities
- [x] `tpt-eng-geometry` / `tpt-math-geometry` integration (feature-gated stubs)
- [x] Round-trip, fuzz, golden-file, property tests
- [ ] README + docs, `cargo publish --dry-run`
- [ ] Note: `tpt-io-citygml` depends on this crate — publish before/alongside it

#### `tpt-io-netcdf` (NetCDF-3 classic + NetCDF-4/HDF5)
- [x] Cargo.toml + `sci-grid-integration` optional feature
- [x] `NetCdfError` enum
- [x] NetCDF-3 classic format parsing
- [x] NetCDF-4 (HDF5-based) format parsing — *h5py-written files: superblock, object headers, links, dataspace, layout, filters*
- [x] Dimension definition extraction
- [x] Variable metadata extraction (units, long_name, valid_range)
- [x] Global attribute extraction
- [~] Chunked reading for large multi-dim arrays (time × depth × lat × lon) — *N-D chunk assembly verified on a 4-D h5py file; the whole variable is still decoded into memory (no streaming API yet)*
- [x] Coordinate variable detection + automatic coordinate mapping
- [x] Deflate/shuffle compression support — *zlib needs the `std` feature*
- [x] Chunking metadata for performance
- [x] `tpt-sci-grid` integration (feature-gated stub)
- [x] Round-trip, fuzz, golden-file, property tests
- [x] README + docs
- [ ] `cargo publish --dry-run` — blocked: `tpt-io-core` dependency not yet on crates.io (see §4 publish order)

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
- [x] README + docs pass
- [ ] `cargo publish --dry-run` — blocked: `tpt-io-core` dependency not yet on crates.io (see §4 publish order)

### Process Engineering

#### `tpt-io-capeopen` (CAPE-OPEN 1.1 / 2.0)
- [x] Cargo.toml + `thermo-integration`, `eng-unit-ops-integration` optional features + Windows-only `com` feature (off by default)
- [x] `CapeOpenError` enum
- [x] Pure-Rust data model for thermodynamic property packages (component lists, phase ID, property calc requests) — no COM required
- [x] Unit operation port mapping (inlet/outlet streams)
- [x] Stream connection validation for flowsheet topology
- [x] `com` feature: FFI-safe wrappers for `ICapeThermoMaterial`, `ICapeThermoCalculation`, `ICapeThermoPropertyPackage` (Windows-only, via `windows` crate)
- [x] CI: verify crate builds with default (no COM) features on Linux/macOS; COM feature validated on a Windows runner
- [x] `tpt-thermo` / `tpt-eng-unit-ops` integration (feature-gated stubs)
- [x] Round-trip, fuzz (non-COM paths), golden-file, property tests
- [x] README + docs (call out COM/Windows caveat prominently)
- [ ] `cargo publish --dry-run` — blocked: `tpt-io-core` dependency not yet on crates.io (see §4 publish order)

#### `tpt-io-sbml` (SBML Level 2 / 3)
- [x] Cargo.toml + `sci-reaction-network-integration`, `sci-ode-integration` optional features
- [x] `SbmlError` enum
- [x] Species / compartment / reaction extraction
- [x] Rate law extraction with MathML expression parsing
- [x] Rule definitions (assignment, rate, algebraic)
- [x] Event handling
- [x] Function definitions
- [x] FBC (flux balance analysis) package extension
- [x] Qual (qualitative models) package extension
- [x] Spatial modeling package extension
- [x] Model validation against SBML schema — *structural and semantic checks in `validate.rs`; not validated against the official XSD*
- [x] Mass balance / charge balance consistency checking
- [x] `tpt-sci-reaction-network` / `tpt-sci-ode` integration (feature-gated stubs)
- [x] Round-trip, fuzz, golden-file, property tests
- [x] README + docs pass
- [ ] `cargo publish --dry-run` — blocked: `tpt-io-core` dependency not yet on crates.io (see §4 publish order)

## 2b. Proposed additional crates (backlog)

Candidate formats identified as gaps. Start only after the unfinished crates in §2 are done.
Every new crate follows the standard sub-checklist (see `tpt-io-dicom`): Cargo.toml + error enum,
parsers, round-trip / fuzz / golden-file / property tests, README + CHANGELOG, `cargo publish --dry-run`.
Each must also be added to the publish order, umbrella features, root README table and CI feature matrix.

### Foundation (unblocks existing work)
- [~] `tpt-io-hdf5` — pure-Rust HDF5 reader (superblock, groups, datasets, chunking, deflate/shuffle); unblocks NetCDF-4, MATLAB v7.3; fold in `hdf5_test` findings — *extracted from `tpt-io-netcdf` with datatype and attribute decoding fixed; README, CHANGELOG, fuzz and tests done; umbrella feature, root README row and CI alloc-lane entry done. Still open: publish dry-run (blocked on crates.io), `hdf5_test` scratch crate not yet folded in (it uses the third-party `hdf5-io` crate; kept for now, needs a decision on removal), MATLAB v7.3 support*
- [x] `tpt-io-core`: shared S-expression tokenizer (`tpt_io_core::sexpr`; EDIF uses it, KiCad still to come)
- [x] `tpt-io-core`: shared STEP Part 21 (ISO 10303-21) tokenizer (`tpt_io_core::step`; IFC uses it)

### Earth Science, GIS & Energy
- [ ] `tpt-io-geo`: WKT / WKB
- [ ] `tpt-io-geo`: KML / KMZ
- [ ] `tpt-io-geo`: GPX
- [ ] `tpt-io-geo`: GeoPackage
- [ ] `tpt-io-geo`: FlatGeobuf
- [ ] `tpt-io-segy` — SEG-Y rev 0/1/2 (seismic; pairs with `tpt-io-welllog`)
- [ ] `tpt-io-pointcloud` — LAS/LAZ point clouds (separate from well-log LAS)
- [ ] `tpt-io-grib` — GRIB2 (meteorology/oceanography; companion to NetCDF)
- [ ] `tpt-io-witsml` — WITSML / RESQML (oil & gas data exchange)

### CAD, Mesh & Civil
- [ ] `tpt-io-dxf` — DXF (ASCII + binary)
- [ ] `tpt-io-mesh` — STL, OBJ, PLY
- [ ] `tpt-io-mesh`: glTF 2.0 / GLB
- [ ] `tpt-io-mesh`: 3MF
- [ ] `tpt-io-landxml` — LandXML
- [ ] `tpt-io-gbxml` — gbXML (building energy)
- [ ] `tpt-io-bcf` — BCF (BIM Collaboration Format)

### Electronics
- [~] `tpt-io-kicad` — `.kicad_pcb` / `.kicad_sch` / `.kicad_sym` — *KiCad 6 PCB nets, footprints, pads, tracks and vias; schematic symbols and wires; symbol libraries; KiCad 5 PCB `module` footprints; umbrella, README and CI wired. Still open: zone fills, schematic labels/junctions, KiCad 5 `.sch` schematics (legacy text format, out of scope so far), and the publish dry-run (blocked on crates.io)*
- [ ] `tpt-io-ipc2581` — IPC-2581
- [ ] `tpt-io-gds` — GDSII
- [ ] `tpt-io-touchstone` — Touchstone `.sNp`
- [ ] `tpt-io-spice`: SPICE `.raw` simulation output
- [ ] `tpt-io-verilog` — structural Verilog netlists

### Medical & Healthcare
- [ ] `tpt-io-hl7v2` — HL7 v2.x (pipe-delimited messages)
- [ ] `tpt-io-edf` — EDF / EDF+ / BDF (EEG, ECG)
- [ ] `tpt-io-fhir` / new crate: C-CDA / CDA
- [ ] `tpt-io-openehr` — openEHR archetypes + compositions
- [ ] `tpt-io-nifti` / new crate: NRRD and MINC

### Process Engineering & Science
- [ ] `tpt-io-dexpi` — DEXPI (P&ID exchange)
- [ ] `tpt-io-automationml` — AutomationML (CAEX)
- [ ] `tpt-io-sbml` / new crate: CellML, SED-ML, COMBINE/OMEX archives
- [ ] `tpt-io-pdb` — PDB / mmCIF
- [ ] `tpt-io-chem` — MOL / SDF
- [ ] `tpt-io-jcamp` — JCAMP-DX (spectroscopy)
- [ ] `tpt-io-mzml` — mzML (mass spectrometry)


## 2c. Next-up fields (priority over §2b backlog)

New domains to tackle next. **Free, openly published specs only** — no proprietary or paid-spec formats.
Never copy spec text, schemas, or unlicensed sample data into the repo; record "spec access / licence"
in each crate README. Each crate follows the standard `tpt-io-dicom` sub-checklist and must be added to
the publish order, umbrella features, root README table and CI feature matrix.
Tags: `[free]` open spec · `[free, licence terms]` free but the publisher's licence/trademark terms apply.

### Power & Energy (open-source formats only)
- [ ] `tpt-io-matpower` — MATPOWER case files `[free]`
- [ ] `tpt-io-opendss` — OpenDSS scripts (EPRI, BSD) `[free]`
- [ ] `tpt-io-gridlabd` — GridLAB-D GLM `[free]`
- Excluded: CIM / IEC 61850 SCL / COMTRADE (paid IEC/IEEE specs), PSS/E RAW (proprietary)

### Automotive, Robotics & Autonomous Driving
- [ ] `tpt-io-mcap` — MCAP `[free]`
- [ ] `tpt-io-rosbag` — ROS bag 1/2 `[free]`
- [ ] `tpt-io-urdf` — URDF + SDFormat `[free]`
- [ ] `tpt-io-opendrive` — ASAM OpenDRIVE `[free, licence terms]`
- [ ] `tpt-io-openscenario` — ASAM OpenSCENARIO `[free, licence terms]`
- [ ] `tpt-io-arxml` — AUTOSAR ARXML `[free, licence terms]`
- [ ] `tpt-io-mdf` — ASAM MDF4 `[free, licence terms]`- Excluded: DBC (proprietary, no official spec)

### Aerospace, Astronomy & Geodesy
- [ ] `tpt-io-fits` — FITS `[free]`
- [ ] `tpt-io-ccsds` — CCSDS OEM / OPM / TDM `[free]`
- [ ] `tpt-io-gnss` — RINEX + SP3 `[free]`
- [ ] `tpt-io-tle` — TLE / orbital elements `[free]`
- [ ] `tpt-io-naif` — NAIF SPICE kernels (named to avoid clash with circuit `tpt-io-spice`) `[free]`
- Excluded: RTCM, NMEA 0183, AIS/IEC 61162 (paid specs)

### Industrial Automation (Industry 4.0)
- [ ] `tpt-io-opcua` — OPC UA NodeSet XML `[free]`
- [ ] `tpt-io-aas` — Asset Administration Shell (IDTA) `[free]`
- [ ] `tpt-io-plcopen` — PLCopen XML (IEC 61131-3 exchange) `[free]`
- [ ] `tpt-io-mtconnect` — MTConnect `[free]`
- [ ] `tpt-io-gcode` — G-code, LinuxCNC / RepRap dialects only `[free]`
- Related: AutomationML, DEXPI in §2b
- Excluded: ISO 6983 (paid)

### Water & Civil Hydraulics
- [ ] `tpt-io-epanet` — EPANET / SWMM `.inp` `[free]` (US EPA, public domain)
- [ ] `tpt-io-waterml` — WaterML 2.0 `[free]`
- [ ] `tpt-io-hecras` — HEC-RAS text geometry/flow files `[free]`

### Digital Pathology & Microscopy
- [ ] `tpt-io-ome` — OME-TIFF + OME-Zarr `[free]`
- [ ] `tpt-io-tiff` — generic tiled/pyramidal TIFF + BigTIFF `[free]`
- [ ] `tpt-io-dicom`: DICOM-WSI (whole-slide imaging) `[free]`
- Excluded: vendor formats (SVS, NDPI, CZI)

### Computational Chemistry & Materials
- [ ] `tpt-io-cif` — CIF `[free]`
- [ ] `tpt-io-chemfile` — Gaussian cube, POSCAR, LAMMPS data, XYZ `[free]`
- Related: PDB/mmCIF, MOL/SDF in §2b

### Maritime & Aviation Navigation
- [ ] `tpt-io-s57` — IHO S-57 / S-101 `[free]` (ENC chart *data* may be separately licensed)
- [ ] `tpt-io-aixm` — AIXM `[free]`
- [ ] `tpt-io-asterix` — ASTERIX (Eurocontrol) `[free]`
- Excluded: ARINC 424 (paid spec, licensed navdata)

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

- [ ] Finish unfinished existing crates (`ifc`, `citygml`, `geo`, `sbml`, `capeopen`, `fhir`) before starting the §2b backlog
- [ ] Licence review per new crate (spec access, trademarks, sample-data rights) recorded in each crate README

- [ ] Decide real crates.io publish order (must respect dependency graph: core → geo → citygml → … → umbrella)
- [ ] Confirm `repository` URL in workspace `Cargo.toml` once the GitHub org/repo is finalized (currently a placeholder)
- [ ] `cargo publish --dry-run` clean for every crate, not just spot-checked ones
- [ ] Tag `v0.1.0` release once all crates are publish-ready
- [ ] Actual `cargo publish` run per crate in dependency order (manual, not automated)
- [ ] Post-publish: verify docs.rs builds succeed for every crate (including feature-gated docs)
