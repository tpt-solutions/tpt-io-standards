# tpt-io

Umbrella crate re-exporting every [`tpt-io`](https://crates.io/crates/tpt-io)
format parser behind opt-in Cargo features.

```toml
[dependencies]
tpt-io = { version = "0.1", features = ["dicom", "ifc"] }
```

Compiling with **no** features is a true zero-cost crate that only re-exports
[`tpt-io-core`]. The feature tree is flat (no transitive feature deps).

## Available features

`dicom`, `fhir`, `nifti`, `ifc`, `citygml`, `spice`, `gerber`, `edif`, `geo`,
`netcdf`, `welllog`, `capeopen`, `sbml`.

Each feature pulls in the corresponding format crate and re-exports it as a
module (e.g. `tpt_io::dicom`). The unified [`IoError`](crate::IoError) wraps
each domain error with the producing format.

## Format detection

[`detect::detect_by_extension`](crate::detect::detect_by_extension) and
[`detect::detect_by_magic`](crate::detect::detect_by_magic) provide lightweight,
dependency-free format sniffing.

## License

Licensed under either of [MIT](../LICENSE-MIT) or [Apache-2.0](../LICENSE-APACHE)
at your option.
