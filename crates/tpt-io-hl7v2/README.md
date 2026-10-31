# tpt-io-hl7v2

A reader and writer for HL7 v2.x messages, the pipe-delimited messages used by most hospital
information systems.

## Spec access and licence

HL7 v2 is published by Health Level Seven International under its licence terms. No standard
text, table or sample message is copied into this crate; the test messages are written for
these tests.

## What it reads

- The delimiters from `MSH`: field, component, repetition, escape and subcomponent characters.
- Segments separated by carriage returns or line feeds. Each segment has an id and raw field texts.
- Typed helpers: `Message::message_type()` (MSH-9), `Message::control_id()` (MSH-10),
  `Message::segment(id)`, and `Message::segments_named(id)` for repeated segments.
- `Segment::field(n)` uses HL7's 1-based numbering. For `MSH`, field 1 is the separator and
  field 2 the encoding characters.

Fields are kept as raw text. Components (`^`), repetitions (`~`) and subcomponents (`&`) are
not split further; the caller does that with the separators in `Message::separators`.

## What it writes

Each segment, its fields joined with the message's field separator, terminated by a carriage
return. Parsing the output gives back the same message.

## What it does not do

- Segment structure is not checked against the version's segment tables.
- Escape sequences (`\F\`, `\S\`, …) are kept as written, not decoded.
- Batch and file headers (`BHS`, `FHS`) are read as ordinary segments.
