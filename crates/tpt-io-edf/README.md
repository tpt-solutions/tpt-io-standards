# tpt-io-edf

A reader for EDF, EDF+ and BDF biosignal recordings (EEG, ECG and polysomnography), and a writer for EDF.

## Spec access and licence

EDF is defined by Kemp et al. (1992); BDF is BioSemi's 24-bit extension. Both are published openly.
No specification text, recording or sample data is copied into this crate. Test recordings are built in
code.

## What it reads

- The 256-byte main header: patient, recording, start date and time, number of records, record duration
  and signal count. A file is BDF when its first byte is `0xFF`.
- The signal headers: label, transducer, physical dimension, physical and digital limits, prefiltering and
  samples per record.
- Raw samples: 16-bit little-endian for EDF, 24-bit signed for BDF, concatenated per signal across records.
- `Signal::to_physical` and `Edf::physical` convert raw samples to physical values linearly between the
  digital and physical limits.

## What it writes

EDF (16-bit) files: header, signal headers, and the data records. Parsing the output gives back the same
header fields and raw samples.

## What it does not do

- BDF writing is reported as `EdfError::Unsupported`.
- EDF+ annotation signals (TAL) are read as ordinary signals; their text is not decoded.
- Record duration is kept as a number and not used to compute timestamps.
