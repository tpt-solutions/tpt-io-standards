# tpt-io-edif

EDIF 2.0.0 / 4.0.0 parser for the `tpt-io` workspace — hierarchical electronic
design interchange format (netlists, libraries, cells, and views).

## Features

- **Tokenizer** (`token`): balanced-parenthesis S-expression lexer with
  symbols, integers/reals, double-quoted strings (with escapes), and `;` comments.
- **Parser** (`parser`): builds a typed [`model::Edif`] documenting libraries,
  cells, NETLIST/SCHEMATIC/LAYOUT views, interface ports (with direction),
  instances, and nets. Implements [`tpt_io_core::Reader`] and
  [`tpt_io_core::Validator`].
- **Flattening** (`flatten`): walks a hierarchical `(design …)` from its top cell
  and produces a single flat netlist via a union-find over port references, so a
  net that meets an instance boundary is correctly merged with the subcell's
  internal connectivity.
- **Round-trip** (`writer`): serializes a typed document back to valid EDIF text.
- **Integration stub**: `em-circuit-integration` feature provides a conversion
  boundary into the (not-yet-published) `tpt-em-circuit` crate.

## Example

```rust
use tpt_io_core::Reader;
use tpt_io_edif::parser::EdifReader;
use tpt_io_edif::model::Edif;

let src = b"(edif top (edifVersion 2 0 0) (edifLevel 0) \
  (library lib (cell c (view v (viewType NETLIST) \
    (interface (port a (direction IN)) (port z (direction OUT))) \
    (contents (net n (joined (portRef a) (portRef z))))))) \
  (design d (cellRef c (libraryRef lib)))";
let edif: Edif = EdifReader::new().parse(src).unwrap();
let flat = tpt_io_edif::flatten::flatten(&edif).unwrap();
assert_eq!(flat.nets.len(), 1);
```

## Status

Per the workspace `todo.md`, EDIF is implemented for the netlist-relevant subset
(library/cell/view/interface/instance/net extraction, hierarchical flattening,
round-trip, fuzz, golden-file, and property tests). `cargo publish --dry-run` is
blocked until `tpt-io-core` is on crates.io (see §4 publish order).
