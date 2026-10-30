# tpt-io-kicad

A reader for KiCad PCB layouts (`.kicad_pcb`) and schematics (`.kicad_sch`),
built on the shared S-expression tokenizer in `tpt-io-core`. It has no
dependencies beyond that crate.

## What it reads (KiCad 6 and later)

- The format `version` and the `generator` that wrote the file.
- PCB: nets (`(net id "name")`) and footprints, with library id, layer,
  position, rotation, `Reference` and `Value` properties.
- Schematic: placed symbols, with library id, position, rotation, `Reference`
  and `Value` properties.

Anything that is well-formed S-expression text but is not a PCB or schematic
(for example a `.kicad_pro` project file) returns `KicadError::Unsupported`.

## What it does not read yet

- KiCad 5 files (`module` footprints and `fp_text` properties).
- Tracks, vias, zones and pads, and schematic wires and labels.
- Project files and symbol or footprint libraries.

## Example

```rust
let bytes = std::fs::read("board.kicad_pcb")?;
let doc = tpt_io_kicad::parse(&bytes)?;
for fp in &doc.footprints {
    println!("{:?} {} at ({}, {})", fp.reference, fp.lib_id, fp.x, fp.y);
}
```

## Testing

The fixtures in `tests/samples/` were written by hand to match the KiCad 6
layout. They are not output from KiCad itself, so they check this reader's
reading of the format, not compatibility with a specific KiCad release.
