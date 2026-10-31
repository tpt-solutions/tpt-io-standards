# tpt-io-matpower

A reader, writer and validator for MATPOWER case files (`case*.m`), the
power-flow case format used by MATPOWER and related tools.

## Spec access and licence

The MATPOWER case format is documented in the MATPOWER User's Manual, which is
distributed under a BSD-style licence. No manual text or case data from that
distribution is copied into this crate. The reader is written from the format's
public description and the field layout of the `bus`, `gen` and `branch`
matrices. Sample cases in the test suite are written for these tests.

## What it reads and writes

- `function mpc = name` header (name is kept).
- `mpc.version`, `mpc.baseMVA`.
- `mpc.bus` (13 columns), `mpc.gen` (10 columns), `mpc.branch` (11 columns;
  columns 12–13 default to ±360 when absent). Extra columns are ignored.
- Any other `mpc.*` field (for example `gencost`, `bus_name`) as a scalar,
  string, numeric matrix or cell array of strings, kept in file order.
- `%` comments, `...` line continuation, and `''` escaped quotes.

Other MATLAB syntax is skipped as an unknown statement and is not preserved.

## Validation

`Case::validate` checks a positive `baseMVA`, unique bus numbers, exactly one
reference bus (type 3), valid bus types, generator and branch endpoints that refer
to existing buses, and 0/1 status flags. It does not run a power-flow solve.

## Example

```rust
let case = tpt_io_matpower::parse(&std::fs::read("case9.m")?)?;
case.validate()?;
println!("{} buses, {} branches", case.bus.len(), case.branch.len());
let text = tpt_io_matpower::write(&case);
```

## Limits

- Cell arrays of nested cells and struct arrays are not supported.
- Numeric precision follows Rust's shortest round-trip `f64` formatting.
- The writer emits the standard columns only; columns beyond the ones listed above
  are not written back.
