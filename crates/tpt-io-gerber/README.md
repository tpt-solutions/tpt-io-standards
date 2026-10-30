# tpt-io-gerber

Gerber (RS-274X / X2 / X3) and Excellon drill-file parser for the `tpt-io`
workspace, targeting PCB fabrication data.

The crate parses Gerber layer definitions — apertures, aperture macros, graphics
operations, X2/X3 attributes, and layer polarity — into a structured
[`GerberFile`], and Excellon drill files into an [`ExcellonFile`]. Both expose a
[`tpt_io_core::Reader`] implementation, syntactic/semantic
[`tpt_io_core::Validator`]s, and a [`tpt_io_core::Writer`] that round-trips
Gerber back to text.

[`GerberFile`]: https://docs.rs/tpt-io-gerber/latest/tpt_io_gerber/model/struct.GerberFile.html
[`ExcellonFile`]: https://docs.rs/tpt-io-gerber/latest/tpt_io_gerber/excellon/struct.ExcellonFile.html
[`tpt_io_core::Reader`]: https://docs.rs/tpt-io-core/latest/tpt_io_core/trait.Reader.html
[`tpt_io_core::Validator`]: https://docs.rs/tpt-io-core/latest/tpt_io_core/trait.Validator.html
[`tpt_io_core::Writer`]: https://docs.rs/tpt-io-core/latest/tpt_io_core/trait.Writer.html

## Quick start

```rust
use tpt_io_core::Reader;
use tpt_io_gerber::parser::GerberReader;
use tpt_io_gerber::model::GerberFile;

let src = b"%FSLAX36Y36*%\n%MOIN*%\n%ADD10C,0.5*%\nD10*\nX0Y0D03*\nX100000Y0D01*\nM02*\n";
let reader = GerberReader::new();
let file: GerberFile = reader.parse(src).unwrap();
assert_eq!(file.objects.len(), 2);
```

## What is supported

- **Aperture definitions** (`%ADD…%`): circles, rectangles, obrounds, regular
  polygons, and macro-referencing apertures, each optionally with a central
  hole.
- **Aperture macros** (`%AM…%`): circle, vector line, outline, polygon, and
  thermal primitives.
- **Graphics operations**: flashes (`D03`), linear draws (`D01`), arcs
  (`G02`/`G03` with `I`/`J`), regions (`G36`…`G37`), and moves (`D02`).
- **Coordinate formats**: `FS` parsing with leading/trailing zero suppression
  and absolute/incremental notation; fixed-point encode/decode.
- **Layer polarity**: `%LP…%` (X1) and `%TF.FilePolarity…%` (X2).
- **X2 / X3 attributes**: file (`%TF…%`), aperture (`%TA…%`), and object
  (`%TO…%`) attributes, including net-list (`N`), component (`C`), pin (`P`),
  function, and design-rule (`DR`) attribution.
- **Design-rule extraction**: [`drc::extract_drc`] reduces a file to trace
  widths, annular rings, and explicit design-rule metadata.
- **Excellon drill files**: tool definitions, through-hole holes, and `G85`
  linear slots, with round-trip serialization.

[`drc::extract_drc`]: https://docs.rs/tpt-io-gerber/latest/tpt_io_gerber/drc/fn.extract_drc.html

## Validation

The [`GerberValidator`] runs a syntactic pass (every referenced aperture is
defined; arcs carry an `I`/`J` center) and a semantic pass (aperture hole
diameter is smaller than its width). Disable either via
[`tpt_io_core::ValidationSettings`].

[`GerberValidator`]: https://docs.rs/tpt-io-gerber/latest/tpt_io_gerber/parser/struct.GerberValidator.html

## Features

| Feature | Default | Effect |
| --- | --- | --- |
| `std` | yes | `std`-only conveniences |
| `alloc` | no | `alloc`-based types without `std` |
| `eng-pcb-integration` | no | Stub conversion into a future `tpt-eng-pcb` PCB layer |
| `eng-geometry-integration` | no | Stub conversion into a future `tpt-eng-geometry` geometry |

The `eng-*` features are integration stubs: they compile and link today so the
feature flags are stable, but the downstream `tpt-eng-*` crates are not yet
published. Only the conversion bodies will need to change when they land.

## Testing

```sh
cargo test -p tpt-io-gerber
cargo +nightly fuzz run gerber
```

## Status

Per the workspace `todo.md`, `tpt-io-gerber` implements RS-274X / X2 / X3 Gerber
parsing, Excellon drill-file parsing, aperture and aperture-macro decoding,
graphics operations, X2/X3 attribute handling, layer polarity, DRC extraction,
round-trip serialization, fuzz, golden-file, and property-based tests. `cargo
publish --dry-run` is blocked until `tpt-io-core` is on crates.io (see §4 publish
order).

## License

Licensed under either of [MIT](../LICENSE-MIT) or [Apache-2.0](../LICENSE-APACHE)
at your option.
