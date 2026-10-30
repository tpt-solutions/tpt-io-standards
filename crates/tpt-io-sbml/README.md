# tpt-io-sbml

SBML (Systems Biology Markup Language) Level 2 / Level 3 parser for the
[`tpt-io`](https://github.com/tpt-solutions/tpt-io-standards) workspace.

Parses SBML documents into a strongly-typed model, with MathML expression
extraction, rule/event/function definitions, the **FBC**, **Qual**, and
**Spatial** package extensions, and syntactic/semantic validation with
charge- and element-balance consistency checking.

## Quick start

```rust
use tpt_io_sbml::{Sbml, SbmlParser};
use tpt_io_core::Reader;

let doc = br#"<?xml version="1.0"?>
<sbml xmlns="http://www.sbml.org/sbml/level2/version4" level="2" version="4">
  <model id="m">
    <listOfSpecies>
      <species id="S" compartment="c" initialConcentration="1"/>
    </listOfSpecies>
  </model>
</sbml>"#;
let sbml = Sbml::parse(doc).unwrap();
assert_eq!(sbml.level, 2);
assert_eq!(sbml.model.species.len(), 1);
```

## Validation

```rust
use tpt_io_sbml::validate_sbml;

let report = validate_sbml(&sbml);
if report.is_ok() {
    // model is structurally and semantically consistent
}
```

For finer control, the [`SbmlParser`] implements `tpt_io_core::Validator`:

```rust
use tpt_io_core::{Reader, Validator};
use tpt_io_sbml::SbmlParser;

let parser = SbmlParser::default();
let sbml = parser.parse(doc)?;
parser.validate_syntactic(&sbml)?; // structure only
parser.validate_semantic(&sbml)?; // meaning (balance, references)
```

Validation performs no XSD/schema validation; it is a set of structural and
consistency checks sufficient to catch most authoring errors (missing
compartments, dangling references, duplicate ids, unbalanced reactions).

[`SbmlParser`]: https://docs.rs/tpt-io-sbml/latest/tpt_io_sbml/parser/struct.SbmlParser.html

## Features

| Feature | Default | Effect |
| --- | --- | --- |
| `std` | yes | Standard library support. |
| `alloc` | yes* | `alloc`-only builds (`no_std` compatible). |
| `sci-reaction-network-integration` | no | Stub conversion into a future `tpt-sci-reaction-network`. |
| `sci-ode-integration` | no | Stub conversion into a future `tpt-sci-ode`. |

\* `alloc` is enabled by `std`.

## Testing

- **Unit tests**: inline in each module.
- **Golden files**: `tests/samples/` (core L2, FBC L3, Qual L3, Spatial L3)
  exercised by `tests/integration.rs`.
- **Property tests**: `tests/proptest.rs` (parser/validator never panic on
  arbitrary bytes; parsing is deterministic).
- **Fuzz**: `cargo +nightly fuzz run sbml` (target in `fuzz/`).

```sh
cargo test -p tpt-io-sbml
cargo +nightly fuzz run sbml
```

> **Round-trip**: this crate is a *parser*; it does not yet include an SBML
> serializer, so writer/round-trip tests are not available. Adding a writer
> would unlock `parse → serialize → parse → equality` coverage.

## Status

Per the workspace `todo.md`, `tpt-io-sbml` implements SBML Level 2/3 document
parsing, MathML expression extraction, FBC / Qual / Spatial package support,
structural and semantic validation, fuzz, golden-file, and property-based tests.
`cargo publish --dry-run` is blocked until `tpt-io-core` is on crates.io
(see §4 publish order).

## License

Licensed under either of [MIT](../LICENSE-MIT) or [Apache-2.0](../LICENSE-APACHE)
at your option.
