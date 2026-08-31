# tpt-io-core

Shared foundation for every `tpt-io-*` format parser: the [`Reader`],
[`Writer`], [`Validator`], and [`Schema`] traits, a structured [`ParseError`]
type with byte-offset and spec-reference reporting, encoding detection, and
optional checksum utilities.

[`Reader`]: https://docs.rs/tpt-io-core/latest/tpt_io_core/trait.Reader.html
[`Writer`]: https://docs.rs/tpt-io-core/latest/tpt_io_core/trait.Writer.html
[`Validator`]: https://docs.rs/tpt-io-core/latest/tpt_io_core/trait.Validator.html
[`Schema`]: https://docs.rs/tpt-io-core/latest/tpt_io_core/trait.Schema.html
[`ParseError`]: https://docs.rs/tpt-io-core/latest/tpt_io_core/struct.ParseError.html

## Quick start

```rust
use tpt_io_core::{Reader, Writer, Validator};

let bytes = b"...";
let reader = /* format-specific reader */;
let value = reader.parse(bytes).unwrap();

let writer = /* format-specific writer */;
let serialized = writer.serialize(&value).unwrap();

// Validate
let report = validator.validate(&value);
assert!(report.is_ok());
```

## Features

| Feature | Default | Effect |
| --- | --- | --- |
| `std` | yes | `std`-only conveniences |
| `alloc` | no | `alloc`-based types without `std` |
| `checksums` | no | CRC32 / MD5 / SHA256 helpers |

## Testing

```sh
cargo test -p tpt-io-core
cargo test -p tpt-io-core --features checksums
cargo +nightly fuzz run tpt-io-core
```

## Status

Foundation crate for the `tpt-io-standards` workspace. Provides the shared
traits, error types, encoding detection, and checksum utilities used by every
format crate. Ready to publish to crates.io.

## License

Licensed under either of [MIT](../LICENSE-MIT) or [Apache-2.0](../LICENSE-APACHE)
at your option.
