# tpt-io-kicad

A reader for KiCad PCB layouts (`.kicad_pcb`), schematics (`.kicad_sch`) and
symbol libraries (`.kicad_sym`), built on the shared S-expression tokenizer in
`tpt-io-core`. It has no dependencies beyond that crate.

## What it reads (KiCad 6 and later)

- The format `version` and the `generator` that wrote the file.
- PCB: nets (`(net id "name")`), footprints with their pads (number, type,
  shape, position, size, net), copper track segments, and vias.
- Schematic: placed symbols (library id, position, rotation, `Reference` and
  `Value`) and wires (their points).
- Symbol libraries: the symbols they define, with default `Reference` and
  `Value`.

Anything that is well-formed S-expression text but is not one of these
documents (for example a `.kicad_pro` project file) returns
`KicadError::Unsupported`.

## What it does not read yet

- KiCad 5 files (`module` footprints and `fp_text` properties).
- Zones and their fills, footprint graphics, and schematic labels and junctions.
- Symbol graphics and pin definitions in libraries.
- Project files and library tables.

## Example

```rust
let bytes = std::fs::read("board.kicad_pcb")?;
let doc = tpt_io_kicad::parse(&bytes)?;
for fp in &doc.footprints {
    println!("{:?} {} at ({}, {})", fp.reference, fp.lib_id, fp.x, fp.y);
    for pad in &fp.pads {
        println!("  pad {} net {:?}", pad.number, pad.net);
    }
}
for t in &doc.tracks {
    println!("track {:?} -> {:?} on {} ({} mm)", t.start, t.end, t.layer, t.width);
}
```

## Testing

The fixtures in `tests/samples/` were written by hand to match the KiCad 6
layout. They are not output from KiCad itself, so they check this reader's
reading of the format, not compatibility with a specific KiCad release.
