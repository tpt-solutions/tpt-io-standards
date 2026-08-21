# tpt-io-dicom

DICOM (Digital Imaging and Communications in Medicine) parser for the
`tpt-io` workspace, implementing PS3.5, PS3.6, PS3.7, and PS3.10 subsets in
pure safe Rust (`#![forbid(unsafe_code)]`, `no_std`-compatible with `alloc`).

## Features

- **Transfer syntaxes**: Implicit VR Little Endian, Explicit VR Little
  Endian, Explicit VR Big Endian — full parse + serialize (round-trip).
- **Sequences of Items** with defined *and* undefined lengths.
- **PS3.10 files**: preamble + `DICM` magic, File Meta group with group-length
  recomputation on write.
- **Tag dictionary** (curated PS3.6 subset) for Implicit-VR inference and
  name/VR lookup; binary-searched static table, zero init cost.
- **UID registry**: SOP classes and transfer syntaxes plus PS3.5 §9 UID
  validation.
- **Pixel data**: native frame slicing, encapsulated fragment extraction,
  pure-Rust **RLE Lossless** codec (PS3.5 Annex G), streaming frame access
  with progress hooks for large CT/MRI volumes. JPEG-family transfer syntaxes
  are detected but decoding requires an external image backend.
- **Anonymization**: practical subset of the PS3.15 Basic Confidentiality
  Profile with stable UID remapping.
- **Hierarchy navigation**: Patient → Study → Series → Instance trees.
- **Core trait integration**: `Reader` / `Writer` / `Validator`
  (syntactic vs. semantic split) from [`tpt-io-core`](../tpt-io-core).

## Usage

```rust
let bytes: Vec<u8> = std::fs::read("image.dcm")?;
let dicom = tpt_io_dicom::file::DicomFile::parse(&bytes)?;
println!("{:?}", dicom.dataset.string(tpt_io_dicom::tag::Tag::new(0x0010, 0x0010)));

// Round-trip back to bytes.
let out = dicom.serialize()?;
assert_eq!(out, bytes);
```

## Feature flags

| Flag | Default | Effect |
|---|---|---|
| `std` | yes | enables `tpt-io-core/std` |
| `alloc` | no | `no_std` + heap |
| `sci-image-integration` | no | `sci_image` bridge module (stub until `tpt-sci-image` publishes) |

## Testing

46 unit/property tests cover round-trips across all three transfer syntaxes,
sequences, encapsulated pixel data, RLE codec fuzz-lite properties, error
paths on truncated input, anonymization, and hierarchy grouping. `cargo-fuzz`
targets live in the workspace `fuzz/` directory.

## Limitations

- Deflated Explicit VR LE is parsed at the meta level only (no inflate).
- JPEG / JPEG-LS / JPEG 2000 pixel data: fragments are extracted; decoding
  needs an external codec.
- Command-set (PS3.7 message) elements are structurally handled but no
  DIMSE message layer is provided.
