# tpt-io-capeopen

CAPE-OPEN 1.1 / 2.0 interface definitions and a pure-Rust thermodynamic property-package
and flowsheet data model for process simulation interoperability.

The crate has two layers:

1. **A pure-Rust, cross-platform data model** (always available) mirroring the
   concepts CAPE-OPEN exposes through its thermo COM interfaces — component lists,
   phases, base properties, property-calculation requests, property packages, and
   material-stream states — plus unit-operation port mapping and flowsheet
   connection validation.
2. **Windows-only COM wrappers** (the `com` feature) that call the CAPE-OPEN
   thermo COM interfaces (`ICapeThermoMaterial`, `ICapeThermoCalculation`,
   `ICapeThermoPropertyPackage`) via the [`windows`](https://crates.io/crates/windows)
   crate.

> ⚠️ **COM / Windows caveat**
>
> The `com` feature is **Windows-only** and **off by default**. On Linux and macOS
> the crate builds with *no* COM dependency and *no* `unsafe` code. Enabling
> `com` requires a Windows host (it binds the `windows` crate and calls into the
> CAPE-OPEN COM runtime), and the vtable layouts must be audited against the
> official CAPE-OPEN type library before production use — see [`src/com.rs`].
> CI validates `com` on a dedicated Windows runner.

## Quick start

```rust
use tpt_io_core::{Reader, Writer};
use tpt_io_capeopen::model::{ComponentList, Phase, PropertyPackage};
use tpt_io_capeopen::parser::PropertyPackageReader;
use tpt_io_capeopen::writer::PropertyPackageWriter;

let mut pp = PropertyPackage::new("PengRobinson", ComponentList::from_ids(&["WATER", "METHANOL"]));
pp.supported_phases.push(Phase::Vapor);
let bytes = PropertyPackageWriter::new().serialize(&pp).unwrap();
let back: PropertyPackage = PropertyPackageReader::new().parse(&bytes).unwrap();
assert_eq!(pp, back);
```

The model is encoded to / decoded from a stable, canonical text form (see
[`writer::encode_property_package`]). CAPE-OPEN has no on-disk file format of its
own — it is a COM interoperability standard — so this text codec is the crate's
round-trippable interchange representation. The reader applies the `tpt-io-core`
syntactic/semantic validation split via `ValidationSettings`.

### Unit-operation ports and flowsheet topology

[`ports`] models material [`Port`]s (inlet / outlet) on a
[`UnitOperation`]; [`flowsheet`] connects them through [`Stream`]s and
[`Flowsheet::validate`]s the connectivity invariants a CAPE-OPEN simulation
environment expects (one stream per outlet→inlet pair, every inlet connected, no
undeclared stream references). Recycle loops are allowed.

## COM interop (Windows-only, `com` feature)

```toml
[dependencies]
tpt-io-capeopen = { version = "0.1", features = ["com"] }
```

```rust,no_run
# #[cfg(all(feature = "com", windows))]
# {
use tpt_io_capeopen::com::CapeThermoPropertyPackage;
// `ptr` is a raw `ICapeThermoPropertyPackage` obtained from a CAPE-OPEN server.
let ptr: *mut core::ffi::c_void = todo!();
let pkg = unsafe { CapeThermoPropertyPackage::from_raw(ptr) };
let model = pkg.to_property_package("MyPackage").unwrap(); // pure-Rust `PropertyPackage`
# }
```

[`com::CapeThermoMaterial`], [`com::CapeThermoCalculation`], and
[`com::CapeThermoPropertyPackage`] wrap the COM interfaces and convert their
SAFEARRAY/VARIANT results into owned Rust types from [`model`]. The wrappers own
the COM reference and `Release` it on drop. **All `unsafe` is confined to
`src/com.rs`.**

## Features

| Feature | Default | Effect |
| --- | --- | --- |
| `std` | yes | `std`-only conveniences (the `Writer`/`Schema` traits, `std::error::Error`). |
| `alloc` | no | `alloc`-based types without `std`. |
| `thermo-integration` | no | Stub conversion into a future `tpt-thermo` property model. |
| `eng-unit-ops-integration` | no | Stub conversion into a future `tpt-eng-unit-ops` model. |
| `com` | no | Windows-only FFI-safe wrappers for the CAPE-OPEN thermo COM interfaces (via `windows`). |

## Testing strategy

Per `spec.txt §6`: round-trip (encode → parse → equality), property-based
(proptest), golden-file (`tests/samples/`), and fuzz (`fuzz/`, non-COM paths).
Run:

```sh
cargo test -p tpt-io-capeopen
cargo test -p tpt-io-capeopen --features thermo-integration,eng-unit-ops-integration
# On Windows:
cargo test -p tpt-io-capeopen --features com
```

## Status

Per the workspace `todo.md`, `tpt-io-capeopen` implements the pure-Rust
thermodynamic property-package data model, unit-operation port modeling,
flowsheet validation, Windows COM wrappers, round-trip serialization, and
fuzz/golden-file/property tests. `cargo publish --dry-run` is blocked until
`tpt-io-core` is on crates.io (see §4 publish order).

## License

Licensed under either of [MIT](../LICENSE-MIT) or [Apache-2.0](../LICENSE-APACHE)
at your option.
