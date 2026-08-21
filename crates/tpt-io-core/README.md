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

## Features

| Feature | Default | Effect |
| --- | --- | --- |
| `std` | yes | `std`-only conveniences |
| `alloc` | no | `alloc`-based types without `std` |
| `checksums` | no | CRC32 / MD5 / SHA256 helpers |

## License

Licensed under either of [MIT](../LICENSE-MIT) or [Apache-2.0](../LICENSE-APACHE)
at your option.
