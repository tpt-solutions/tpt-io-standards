# AGENTS.md

## Workspace

- Rust workspace: 15 format crates under `crates/`, plus `hdf5_test` (experimental, not published).
- `tpt-io-core` is the foundation; every domain crate depends on it.
- `tpt-io` is the umbrella crate — publish it last.
- Publish order: `core → geo → citygml → ifc → fhir → dicom → nifti → spice → gerber → edif → netcdf → welllog → capeopen → sbml → tpt-io-matpower → umbrella`.
- MSRV: **1.75**, edition **2021**. `clippy.toml` and `[workspace.package]` enforce this.

## Commands

- Full check: `cargo check --workspace`
- Full test: `cargo test --workspace`
- Single crate: `cargo test -p tpt-io-dicom`
- Single test: `cargo test -p tpt-io-dicom parser::tests::parse_full_dicom`
- Lint: `cargo clippy --workspace --all-targets`
- Lint all features: `cargo clippy --workspace --all-targets --all-features`
- Format check: `cargo fmt --all -- --check`
- Auto-fix clippy: `cargo clippy --fix --allow-dirty --lib -p <crate>`
- Doc tests: `cargo test --workspace --doc`

## CI expectations

CI runs with `RUSTFLAGS: -Dwarnings`, so **warnings are errors**. Keep the workspace warning-free:
- `cargo fmt --all -- --check`
- `cargo clippy --workspace`
- `cargo test --workspace`
- `cargo test --workspace --all-features`
- Minimal zero-cost build: `cargo build -p tpt-io-core --no-default-features && cargo build -p tpt-io --no-default-features`
- MSRV check: `cargo build --workspace` with `rust-toolchain@1.75.0`

## Style

- `rustfmt.toml`: `edition = "2021"`, `max_width = 100`, `use_small_heuristics = "Max"`.
- All crates are `#![forbid(unsafe_code)]` except `tpt-io-capeopen/src/com.rs` (Windows COM, gated behind `com` feature).
- `no_std` + `alloc` + `std` feature split is standard. Domain crates require `alloc`; only `tpt-io-core` and the zero-cost `tpt-io` umbrella build without `alloc`.

## Repo-specific gotchas

- **DICOM source lives in `crates/tpt-io-dicom/src/`**, not the repo root. The orphaned `src/` directory was removed in a prior refactor.
- **Integration features are stubs**: `sci-image-integration`, `eng-geometry-integration`, `em-circuit-integration`, etc. compile today but depend on unpublished crates. Do not remove the stub modules.
- **Golden-file samples**: `tests/samples/` directories exist but may be empty or contain unlicensed data. Check `tests/samples/README.md` before adding files. DICOM samples are gitignored until licensing is confirmed.
- **`hdf5_test`** is a separate binary crate for HDF5 interop experiments. It is in the workspace but not part of the publish order.
- **Windows COM**: only `tpt-io-capeopen` has a `com` feature. It requires `windows` crate and a Windows runner. Do not enable it on Linux/macOS.

## Docs

- Every crate has a `README.md` and `CHANGELOG.md`.
- Update `CHANGELOG.md` when making user-visible changes.
- Keep crate tables in the root `README.md` in sync with `Cargo.toml`.
