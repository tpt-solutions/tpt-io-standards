# tpt-io-gridlabd

A reader, writer and validator for GridLAB-D model files (`.glm`), the input
language of the GridLAB-D distribution-system simulator.

## Spec access and licence

The GLM syntax is taken from the GridLAB-D user documentation, distributed under
a BSD-style licence. No manual text or example models from that distribution are
copied into this crate. Test models are written for these tests.

## What it reads

- `clock`, `module` and `class` blocks, each with its `name value;` properties.
- `object class { … }` blocks, including objects nested inside other objects.
- `#` and `@` directives, kept as written.
- `//` line comments and `/* … */` block comments.
- Property values as raw text, so `100+50j` and quoted timestamps are preserved.

## Validation

`Glm::validate` checks that object names are unique and that every `parent`
property refers to a named object in the model.

## What it does not do

- It does not follow `#include`, expand `#define` macros, or evaluate expressions.
- It does not run the simulation or check property names against the class schema.
- Objects inside `module` or `clock` blocks are rejected.

## Example

```rust
let glm = tpt_io_gridlabd::parse(&std::fs::read("feeder.glm")?)?;
glm.validate()?;
for node in glm.objects_of_class("node") {
    println!("{:?} {:?}", node.name(), node.prop("nominal_voltage"));
}
```
