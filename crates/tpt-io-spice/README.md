# tpt-io-spice

SPICE netlist parser for the `tpt-io` workspace, covering the HSPICE, LTSpice,
Ngspice, and PSpice dialects.

The crate parses element cards (`R`/`L`/`C`/`V`/`I`/`D`/`Q`/`M`/`E`/`F`/`G`/`H`/`T`/`X`/`B`),
`.model` definitions, `.subckt`/`.ends` blocks, and analysis commands
(`.tran`/`.ac`/`.op`/`.dc`/`.noise`) into a structured [`SpiceNetlist`]. Values
with SI suffixes (`k`, `u`, `n`, `p`, `meg`, …) are scaled to their numeric
form. Both a [`tpt_io_core::Reader`] and a [`tpt_io_core::Writer`] (round-trip)
are provided, along with syntactic/semantic [`tpt_io_core::Validator`]s.

[`SpiceNetlist`]: https://docs.rs/tpt-io-spice/latest/tpt_io_spice/model/struct.SpiceNetlist.html
[`tpt_io_core::Reader`]: https://docs.rs/tpt-io-core/latest/tpt_io_core/trait.Reader.html
[`tpt_io_core::Writer`]: https://docs.rs/tpt-io-core/latest/tpt_io_core/trait.Writer.html
[`tpt_io_core::Validator`]: https://docs.rs/tpt-io-core/latest/tpt_io_core/trait.Validator.html

## Quick start

```rust
use tpt_io_core::Reader;
use tpt_io_spice::parser::SpiceReader;

let netlist = b"RC filter\nR1 in out 1k\nC1 out 0 100n\n.end\n";
let reader = SpiceReader::new();
let net = reader.parse(netlist).unwrap();
assert!(net.elements.len() >= 2);
```

## What is supported

- **Dialect detection** is implicit: the parser accepts the common card syntax
  shared across HSPICE / LTSpice / Ngspice / PSpice without dialect-specific
  switches.
- **Passive components** (`R`/`L`/`C`) with SI-suffixed values (`1k`, `2.2u`,
  `1MEG`) and temperature coefficients (`TC=`/`TC1=`/`TC2=`).
- **Active devices** (`D`, `Q`, `M`) and **sources** (`V`, `I`, `E`, `F`, `G`,
  `H`), **transmission lines** (`T`), and behavioral sources (`B`).
- **`.model` cards** for diodes, BJTs, MOSFETs, JFETs, and others, with
  parameter assignment parsing.
- **`.subckt` hierarchical expansion** with formal parameters and parameter
  substitution (see [`expand::expand_netlist`]).
- **Analysis commands** (`.tran`/`.ac`/`.op`/`.dc`/`.noise`) and a global
  `.temp` / `.options temp=` simulation temperature.
- **Temperature-dependent values**: [`Element::value_at_temperature`] resolves a
  passive element's effective value at an arbitrary temperature.

[`expand::expand_netlist`]: https://docs.rs/tpt-io-spice/latest/tpt_io_spice/expand/fn.expand_netlist.html
[`Element::value_at_temperature`]: https://docs.rs/tpt-io-spice/latest/tpt_io_spice/model/struct.Element.html#method.value_at_temperature

## Validation

The [`SpiceValidator`] runs a syntactic pass (every `R`/`L`/`C` has a value;
every `.model` has a device) and a (currently empty) semantic pass. Disable
either via [`tpt_io_core::ValidationSettings`].

[`SpiceValidator`]: https://docs.rs/tpt-io-spice/latest/tpt_io_spice/parser/struct.SpiceValidator.html

## Features

| Feature | Default | Effect |
| --- | --- | --- |
| `std` | yes | `std`-only conveniences |
| `alloc` | no | `alloc`-based types without `std` |
| `em-circuit-integration` | no | Stub conversion into a future `tpt-em-circuit` circuit |

The `em-circuit-integration` feature is an integration stub: it compiles and
links today so the feature flag is stable, but the downstream `tpt-em-circuit`
crate is not yet published. Only the conversion body will need to change when it
lands.

## Testing

```sh
cargo test -p tpt-io-spice
cargo +nightly fuzz run spice
```

## Status

Per the workspace `todo.md`, `tpt-io-spice` implements SPICE netlist parsing for
HSPICE / LTSpice / Ngspice / PSpice dialects, passive and active device cards,
`.model` / `.subckt` / analysis commands, SI-suffix value scaling, temperature
coefficients, round-trip serialization, fuzz, golden-file, and property-based
tests. `cargo publish --dry-run` is blocked until `tpt-io-core` is on crates.io
(see §4 publish order).

## License

Licensed under either of [MIT](../LICENSE-MIT) or [Apache-2.0](../LICENSE-APACHE)
at your option.
