# fuzz

`cargo-fuzz` targets for `tpt-io-dicom`. Requires a nightly toolchain and
[cargo-fuzz](https://github.com/rust-fuzz/cargo-fuzz):

```sh
cargo +nightly install cargo-fuzz
cargo +nightly fuzz run parse_dataset
cargo +nightly fuzz run parse_part10_file
```

Invariant under test: parsing arbitrary bytes never panics — it either
succeeds or returns a structured `DicomError`.
