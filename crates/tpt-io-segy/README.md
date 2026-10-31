# tpt-io-segy

A reader and writer for SEG-Y seismic data files (`.sgy`, `.segy`), the standard format for exchanging
seismic traces.

## Spec access and licence

SEG-Y is published by the Society of Exploration Geophysicists. No specification text or survey data is
copied into this crate; the test files are built in code.

## What it reads

- The 3200-byte textual header, kept as bytes (EBCDIC or ASCII; not translated).
- The 400-byte binary header: sample interval, samples per trace, and the data format code. The whole
  header is kept, so unused fields round-trip.
- Every trace: its 240-byte header (kept as bytes) and its samples decoded to `f32`:
  - format 1, IBM 32-bit float;
  - format 2, 32-bit signed integer;
  - format 3, 16-bit signed integer;
  - format 5, IEEE 32-bit float;
  - format 8, 8-bit signed integer.

## What it writes

The text header, the binary header (with the sample interval, samples per trace and format code 5), and
each trace as its original header and IEEE 32-bit samples. Parsing the output gives back the same traces.

## What it does not do

- Other format codes (for example 4-bit or 64-bit fixed point) are reported as unsupported.
- Trace-header fields are kept as bytes; they are not decoded into named values.
- Extended textual headers and trace-level sample intervals that differ from the binary header are not
  interpreted.
- Writing always uses IEEE 32-bit floats; other formats are not written.
