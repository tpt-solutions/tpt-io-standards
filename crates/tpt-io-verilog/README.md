# tpt-io-verilog

A reader and writer for structural Verilog netlists, the gate-level output of synthesis and
place-and-route tools.

## Spec access and licence

Verilog is standardised as IEEE 1364, which IEEE makes available under its terms. No standard
text is copied into this crate; the grammar subset implemented here follows the structural
module syntax, and the test netlists are written for these tests.

## What it reads

- `module name (ports);`, with the header port list kept in order.
- ANSI-style port declarations in the header (`input wire clk`).
- `input`, `output` and `inout` declarations with optional `[msb:lsb]` ranges, several names
  per declaration.
- `wire` declarations with optional ranges.
- Instances with named connections (`.A(net)`) or positional ones. An empty named connection
  (`.sum()`) is kept as an empty net.
- `assign lhs = rhs;` between two names.
- `//` and `/* */` comments, escaped identifiers (`\name `), and sized literals such as `1'b0`.

## What it rejects

Constructs outside the structural subset return `VerilogError::Unsupported`. Examples are
`always`, `initial`, `reg`, `parameter`, `function`, `task`, `generate` and module parameters.
They are not approximated.

## What it writes

Each module with its header, declarations, assigns and instances. Parsing the output gives
back the same netlist.

## Limits

- Expressions are limited to a single name on each side of an `assign`.
- Instance and net names are not checked against the cell library or for uniqueness.
