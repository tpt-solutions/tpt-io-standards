# tpt-io-opendss

A reader and writer for OpenDSS scripts (`.dss`), the text command language of
the EPRI OpenDSS distribution-system simulator.

## Spec access and licence

The command syntax is taken from the OpenDSS user documentation, which EPRI
publishes under a BSD-style licence. No documentation text or example circuits
from that distribution are copied into this crate; the fixtures in the tests are
written for them.

## What it reads

- Commands split into verb, element class and name (`New Line.L1`), positional
  arguments, and `key=value` properties.
- `!` comments (outside quotes), `//` comment lines, and `~` continuation lines.
- Values in quotes, `[ ]` arrays and `( )` groups that contain spaces. They are kept
  exactly as written.
- `Script::elements(class)` returns the `New` and `Edit` commands for one class,
  matched case-insensitively.

## What it does not do

- It does not evaluate commands, solve power flow, or check that properties are
  valid for an element class. It reads the script; it does not run the circuit.
- `Redirect`, `Compile` and similar commands are kept as commands. Included files
  are not followed.

## Example

```rust
let dss = tpt_io_opendss::parse(&std::fs::read("feeder.dss")?)?;
for line in dss.elements("Line") {
    println!("{:?} length={:?}", line.name, line.prop("length"));
}
```
