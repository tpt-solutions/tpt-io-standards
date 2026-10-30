# tpt-io

Umbrella crate re-exporting every [`tpt-io`](https://crates.io/crates/tpt-io)
format parser behind opt-in Cargo features.

## Quick start

```toml
[dependencies]
tpt-io = { version = "0.1", features = ["dicom", "ifc"] }
```

```rust
use tpt_io::dicom;
use tpt_io::ifc;
```

Compiling with **no** features is a true zero-cost crate that only re-exports
[`tpt-io-core`]. The feature tree is flat (no transitive feature deps).

## Available features

| Feature | Default | Description |
| --- | --- | --- |
| `std` | yes | Standard library support. |
| `alloc` | no | `alloc`-based types without `std`. |
| `dicom` | no | Re-exports `tpt-io-dicom`. |
| `fhir` | no | Re-exports `tpt-io-fhir`. |
| `nifti` | no | Re-exports `tpt-io-nifti`. |
| `ifc` | no | Re-exports `tpt-io-ifc`. |
| `citygml` | no | Re-exports `tpt-io-citygml`. |
| `spice` | no | Re-exports `tpt-io-spice`. |
| `gerber` | no | Re-exports `tpt-io-gerber`. |
| `edif` | no | Re-exports `tpt-io-edif`. |
| `geo` | no | Re-exports `tpt-io-geo`. |
| `netcdf` | no | Re-exports `tpt-io-netcdf`. |
| `welllog` | no | Re-exports `tpt-io-welllog`. |
| `capeopen` | no | Re-exports `tpt-io-capeopen`. |
| `sbml` | no | Re-exports `tpt-io-sbml`. |

Each feature pulls in the corresponding format crate and re-exports it as a
module (e.g. `tpt_io::dicom`). The unified [`IoError`](crate::IoError) wraps
each domain error with the producing format.

## Format detection

[`detect::detect_by_extension`](crate::detect::detect_by_extension) and
[`detect::detect_by_magic`](crate::detect::detect_by_magic) provide lightweight,
dependency-free format sniffing.

## Testing

```sh
cargo test -p tpt-io
# Test a specific format feature:
cargo test -p tpt-io --features dicom,fhir
cargo +nightly fuzz run tpt-io
```

## Status

Umbrella crate with feature-gated re-exports for all format crates in the
workspace. Ready to publish alongside the format crates once `tpt-io-core` is on
crates.io. `cargo publish --dry-run` is blocked until `tpt-io-core` is on
crates.io (see §4 publish order).

## License

Licensed under either of [MIT](../LICENSE-MIT) or [Apache-2.0](../LICENSE-APACHE)
at your option.
