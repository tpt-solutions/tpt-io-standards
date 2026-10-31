# tpt-io-mdf

Reader for ASAM MDF 4 measurement files, the block-based format that stores automotive and test-bench
recordings.

## Spec access and licence

MDF 4 is specified by ASAM and published openly. No specification text is copied into this crate. The reader is
checked against files written by asammdf (`tests/gen_mdf.py` regenerates them), so its block offsets and record
decoding agree with an independent implementation.

## What it reads

- The identification block (version) and the header block.
- Data groups (`DG`), channel groups (`CG`) with their record id, cycle count and record length, and the channel
  chain (`CN`), including each channel's name and unit (`TX` blocks).
- Data in a plain `DT` block, a linked `DL` list, or a deflate `DZ` block, including transposed deflate blocks.
- Integer (unsigned and signed) and IEEE float channels of up to 64 bits, read as `f64` values. Byte-aligned big-endian
  values and little-endian values at any bit offset are decoded.
- Several channel groups in one data group, with records routed by their record id.

## What it does not do

- Conversions (`CC` blocks) are not applied; the values are the raw ones.
- String, byte-array and variable-length (VLSD) channels are skipped.
- Big-endian files (default byte order other than little-endian) are rejected.
- Sorted-group invalidation bytes are skipped, not interpreted.
- Writing MDF files is not supported.
