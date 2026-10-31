# tpt-io-standards

[![License: MIT OR Apache-2.0](https://img.shields.io/badge/License-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE-MIT)

A pure-Rust, feature-gated workspace for parsing, validating, and serializing
industry-standard data formats. Each format lives in its own crate under
`crates/`, sharing a common foundation ([`tpt-io-core`]) so that every parser
offers the same error-reporting, encoding-detection, and validation ergonomics.

Repository: <https://github.com/tpt-solutions/tpt-io-standards>

All parsers are implemented from scratch in pure Rust (no C/C++ FFI such as
libgdal, dcmtk, or netcdf-c), keeping them cross-platform, memory-safe, and
WASM-compatible. Parsers aim for zero-copy / low-allocation reading where
possible, with strict schema validation and structured, byte-offset-aware
errors.

## Workspace layout

| Crate | Format(s) | Domain |
| --- | --- | --- |
| `tpt-io-core` | shared traits, errors, encoding & checksum utils | foundation |
| `tpt-io-dicom` | DICOM (PS3.5/6/7/10) | Medical |
| `tpt-io-fhir` | HL7 FHIR R4/R5 (JSON + XML) | Medical |
| `tpt-io-nifti` | NIfTI-1 / NIfTI-2 | Medical |
| `tpt-io-ifc` | IFC 4.0 / 4.3 (STEP-PF) | Construction |
| `tpt-io-citygml` | CityGML 2.0 / 3.0 | Construction |
| `tpt-io-spice` | SPICE (HSPICE/LTSpice/Ngspice/PSpice dialects) | Electronics |
| `tpt-io-gerber` | Gerber X2/X3 + Excellon | Electronics |
| `tpt-io-edif` | EDIF 2.0.0 / 4.0.0 | Electronics |
| `tpt-io-kicad` | KiCad 6+ PCB, schematic and symbol library (S-expression) | Electronics |
| `tpt-io-geo` | GeoJSON / Shapefile / GeoTIFF | Earth & GIS |
| `tpt-io-hdf5` | HDF5 reader (superblock, groups, chunked data, deflate/shuffle) | Foundation |
| `tpt-io-netcdf` | NetCDF-3 / NetCDF-4 (HDF5) | Earth & GIS |
| `tpt-io-welllog` | LAS 2.0/3.0 + DLIS | Earth & GIS |
| `tpt-io-capeopen` | CAPE-OPEN 1.1 / 2.0 | Process |
| `tpt-io-sbml` | SBML Level 2 / 3 | Process |
| `tpt-io-mdf` | ASAM MDF 4 measurement files: channel groups, channels and values (plain, linked, deflate) | Automotive & Robotics |
| `tpt-io-grib` | GRIB edition 2 gridded weather fields (latitude/longitude, simple packing) | Earth & GIS |
| `tpt-io-naif` | NAIF SPICE ephemeris kernels: DAF/SPK, Chebyshev types 2 and 3 (`.bsp`) | Space & Geodesy |
| `tpt-io-omex` | COMBINE archives (`.omex`) and SED-ML experiment summaries | Science & Materials |
| `tpt-io-bcf` | BIM Collaboration Format 2.1 projects: topics, comments and viewpoint references (`.bcf`, `.bcfzip`) | Construction & CAD |
| `tpt-io-threemf` | 3D Manufacturing Format packages (`.3mf`) | Construction & CAD |
| `tpt-io-mesh` | Triangle meshes: STL, Wavefront OBJ, PLY | Construction & CAD |
| `tpt-io-gnss` | GNSS observations (RINEX 3) and precise orbits (SP3), read only | Earth & GIS |
| `tpt-io-hecras` | HEC-RAS river geometry files (read only) | Water & Hydraulics |
| `tpt-io-ome` | OME-XML, OME-TIFF and OME-Zarr microscopy metadata | Earth & GIS |
| `tpt-io-tiff` | TIFF images (uncompressed and LZW strips) | Earth & GIS |
| `tpt-io-pointcloud` | ASPRS LAS point clouds, uncompressed (`.las`) | Earth & GIS |
| `tpt-io-segy` | SEG-Y seismic traces (`.sgy`, `.segy`) | Science & Materials |
| `tpt-io-gds` | GDSII IC layout streams (`.gds`) | Electronics |
| `tpt-io-edf` | EDF, EDF+ and BDF biosignal recordings | Healthcare |
| `tpt-io-openehr` | openEHR compositions, RM JSON (read only) | Healthcare |
| `tpt-io-nrrd` | NRRD raster data (`.nrrd`, `.nhdr`) | Healthcare & Imaging |
| `tpt-io-cellml` | CellML models (XML, read only) | Science & Materials |
| `tpt-io-dexpi` | DEXPI P&ID components (XML, read only) | Process & Chemical |
| `tpt-io-automationml` | AutomationML CAEX plant hierarchies (read only) | Manufacturing |
| `tpt-io-witsml` | WITSML well trajectories (XML, read only) | Energy & Oil-Gas |
| `tpt-io-ipc2581` | IPC-2581 PCB design data (XML, read only) | Electronics |
| `tpt-io-cda` | HL7 Clinical Document Architecture documents (read only) | Healthcare |
| `tpt-io-gbxml` | gbXML building-energy models (read only) | Construction & CAD |
| `tpt-io-landxml` | LandXML surfaces, units and alignments (civil) | Construction & CAD |
| `tpt-io-mzml` | mzML mass spectrometry spectra (read only) | Science & Materials |
| `tpt-io-jcamp` | JCAMP-DX spectroscopy files (`.jdx`, `.dx`) | Science & Materials |
| `tpt-io-chem` | MDL molfiles (V2000) and SDF records (`.mol`, `.sdf`) | Science & Materials |
| `tpt-io-pdb` | Protein Data Bank fixed-column files (`.pdb`) | Science & Materials |
| `tpt-io-dxf` | AutoCAD DXF drawings (ASCII) | Construction & CAD |
| `tpt-io-hl7v2` | HL7 v2.x pipe-delimited messages | Healthcare |
| `tpt-io-verilog` | Structural Verilog netlists (`.v`) | Electronics |
| `tpt-io-touchstone` | Touchstone network parameter files (`.sNp`) | Electronics |
| `tpt-io-chemfile` | Molecular and crystal structure files (XYZ, cube, POSCAR, LAMMPS) | Science & Materials |
| `tpt-io-cif` | Crystallographic CIF 1.1 and mmCIF (STAR syntax) | Science & Materials |
| `tpt-io-asterix` | ASTERIX surveillance data (binary), CAT048 subset | Aviation & Maritime |
| `tpt-io-aixm` | AIXM 5.1 aeronautical features (XML, read only) | Aviation & Maritime |
| `tpt-io-opcua` | OPC UA NodeSet2 XML information models (read only) | Manufacturing |
| `tpt-io-aas` | Asset Administration Shell environments, JSON (read only) | Manufacturing |
| `tpt-io-plcopen` | PLCopen XML IEC 61131-3 projects (read only) | Manufacturing |
| `tpt-io-mtconnect` | MTConnect devices and streams (XML, read only) | Manufacturing |
| `tpt-io-waterml` | OGC WaterML 2.0 hydrological time series (XML) | Water & Hydraulics |
| `tpt-io-epanet` | EPANET and SWMM network input files (`.inp`) | Water & Hydraulics |
| `tpt-io-gcode` | G-code programs for CNC and 3D printers (`.nc`, `.gcode`) | Manufacturing |
| `tpt-io-fits` | FITS images and HDUs (`.fits`) | Space & Geodesy |
| `tpt-io-ccsds` | CCSDS orbit data messages in KVN form (`.oem`, `.opm`, `.tdm`) | Space & Geodesy |
| `tpt-io-tle` | Two-line element sets (TLE orbital elements) | Space & Geodesy |
| `tpt-io-arxml` | AUTOSAR XML packages and elements (`.arxml`), read-only | Automotive & Robotics |
| `tpt-io-openscenario` | ASAM OpenSCENARIO 1.x scenarios (`.xosc`), read-only | Automotive & Robotics |
| `tpt-io-opendrive` | ASAM OpenDRIVE road networks (`.xodr`) | Automotive & Robotics |
| `tpt-io-urdf` | URDF and SDFormat robot descriptions (`.urdf`, `.sdf`) | Automotive & Robotics |
| `tpt-io-rosbag` | ROS 1 bags, format 2.0 (`.bag`) | Automotive & Robotics |
| `tpt-io-mcap` | MCAP robotics logs (`.mcap`) | Automotive & Robotics |
| `tpt-io-gridlabd` | GridLAB-D models (`.glm`) | Power & Energy |
| `tpt-io-opendss` | OpenDSS scripts (`.dss`) | Power & Energy |
| `tpt-io-matpower` | MATPOWER case files (`case*.m`) | Power & Energy |
| `tpt-io` | umbrella re-export of all formats | umbrella |

## Using the umbrella crate

Downstream Tier-2 repositories enable only the formats they need:

```toml
[dependencies]
tpt-io = { version = "0.1", features = ["dicom", "ifc"] }
```

Compiling `tpt-io` with **no** features is a true zero-cost crate that only
re-exports [`tpt-io-core`]. The feature tree is intentionally flat (no
transitive feature dependencies) to avoid Cargo feature-unification surprises.

Individual format crates can also be depended on directly:

```toml
[dependencies]
tpt-io-dicom = "0.1"
```

## Feature flags on `tpt-io-core`

| Feature | Default | Effect |
| --- | --- | --- |
| `std` | yes | `std`-only conveniences (e.g. `std::io` adapters) |
| `alloc` | no | `alloc`-based types without `std` |
| `checksums` | no | CRC32 / MD5 / SHA256 helpers |

## Design principles

- **Structured errors** — every parse error carries the byte offset, the
  expected-vs-actual token, a reference into the format specification, and a
  slice of surrounding bytes (see `tpt_io_core::ParseError`).
- **Two-phase validation** — syntactic (is it structurally valid?) and
  semantic (does it make sense?) checks, split via `Validator` and toggleable
  with `with_syntactic_validation` / `with_semantic_validation`.
- **Zero-copy where possible** — parsers prefer `&str` / `&[u8]` slices into
  the input buffer and support streaming for very large files.
- **Extensible** — the `FormatExtension` trait lets users add custom tags or
  dialects without forking a crate.

## MSRV policy

The minimum supported Rust version (MSRV) is **1.75**, declared in
`[workspace.package]` (`rust-version`). Crates must therefore target edition
2021. We will only raise the MSRV in a minor (or major) release, and the bump
will be called out in the changelog. CI always builds against the pinned
`rust-version` in addition to stable.

## Building & testing

```sh
cargo build --workspace
cargo test  --workspace
cargo test  --workspace --all-features
cargo clippy --workspace --all-targets --all-features
cargo fmt   --all -- --check
```

## License

Licensed under either of [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE) at
your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you shall be dual licensed as above, without any
additional terms or conditions.
