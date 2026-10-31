# tpt-io-nrrd

A reader and writer for NRRD (Nearly Raw Raster Data) files, the header-plus-data raster format used
in medical imaging and scientific visualisation.

## Spec access and licence

NRRD is defined by its format specification, published openly by Gordon Kindlmann and the Teem project.
No text or sample data is copied into this crate; the test files are written in code.

## What it reads

- The magic line (`NRRD0001` to `NRRD0005` and later).
- Header fields as `name: value`, lower-cased names, and `key:=value` pairs. Comment lines (`#`) are skipped.
- The data after the blank line, as stored.
- Typed sample decoding through `Nrrd::values` for `raw` data: `uchar`, `char`, `short`, `ushort`,
  `int`, `uint`, `float` and `double`, honouring `endian`; and for `ascii` data.
- `Nrrd::sizes` and `Nrrd::sample_count` from the `sizes` field.

## What it writes

The magic, the header fields in order, a blank line and the data bytes. Parsing the output gives back
the same file.

## What it does not do

- External data files (`data file:`) are reported as `NrrdError::Unsupported`.
- `gzip` and `bzip2` encodings are reported as unsupported; no decompression is attempted.
- `hex` encoding and 64-bit integer types are not decoded.
- Spacings, space directions and the other spatial fields are kept as header text, not interpreted.
