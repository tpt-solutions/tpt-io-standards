# tpt-io-fits

A reader and writer for FITS (Flexible Image Transport System) files, the standard
format for astronomical images and tables.

## Spec access and licence

The FITS standard is published by the IAU FITS Working Group and the FITS Support
Office and is openly available. No text, keyword dictionary or sample file from those
sources is copied into this crate; the card layout is implemented from the standard's
80-column format, and the test files are built in code.

## What it reads

- Header cards in 2880-byte blocks, up to `END`: keyword, value and comment, with
  logical (`T`/`F`), integer, real (`E` or `D` exponent) and quoted string values
  (doubled quotes resolved).
- Commentary cards (`COMMENT`, `HISTORY`, blank keyword) as text.
- Every HDU in the file: the primary and any extensions. Data sizes come from
  `BITPIX`, `NAXIS`, `NAXISn`, `PCOUNT` and `GCOUNT`.
- Image pixels for `BITPIX` 8, 16, 32, 64, -32 and -64, with `BZERO` and `BSCALE`
  applied.

## What it writes

Each HDU's cards, a terminating `END`, and its data, padded to block boundaries. The
round trip gives back the same cards and bytes.

## What it does not do

- Binary and ASCII table HDUs are kept as raw bytes; columns (`TFORMn`, `TTYPEn`)
  are not decoded.
- Complex values, random groups and tile compression are not interpreted.
- Writing computes `NAXISn` only from the caller's `Hdu::image` or card list; data
  length is not checked against the header on output.
