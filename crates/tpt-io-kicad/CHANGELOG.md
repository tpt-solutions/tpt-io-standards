# Changelog

## Unreleased

- Initial reader on the shared `tpt_io_core::sexpr` tokenizer: format version
  and generator, PCB nets and footprints, schematic symbols.
- Adds footprint pads, PCB tracks and vias, schematic wires, and `.kicad_sym`
  symbol libraries.
- Reads KiCad 5 PCB files: `module` footprints, `fp_text` reference and value,
  unquoted pad numbers, and the `host` field in place of `generator`.
