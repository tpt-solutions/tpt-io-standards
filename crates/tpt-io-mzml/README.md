# tpt-io-mzml

A read-only reader for mzML mass-spectrometry files: spectra with their MS level, scan time, and
m/z and intensity arrays.

## Spec access and licence

mzML is defined by the HUPO Proteomics Standards Initiative and published openly; the
controlled vocabulary is published by PSI. No specification text or data file is copied into this
crate; the tests use a spectrum built in code.

## What it reads

- The run `id`.
- Each `spectrum`: `index`, `id`, MS level (`MS:1000511`), scan start time (`MS:1000016`, with
  `unitName` `minute` or `second`, returned in seconds).
- Each `binaryDataArray`: the m/z array (`MS:1000514`) and the intensity array (`MS:1000515`),
  decoded from base64 as little-endian 32-bit (`MS:1000521`) or 64-bit (`MS:1000523`) floats.

## What it does not do

- Read only.
- zlib-compressed arrays (`MS:1000574`) are reported as `MzmlError::Unsupported`; no
  decompression is attempted here.
- Other array types, chromatograms, precursor information, and indexed mzML offsets are not
  read.
- Spectra are read into memory in full.
