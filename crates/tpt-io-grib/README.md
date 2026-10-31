# tpt-io-grib

Reader and writer for GRIB edition 2 messages (WMO FM 92): gridded weather, climate and ocean fields, one field per
message.

## Spec access and licence

GRIB is defined by the WMO Manual on Codes (FM 92). No specification text is copied into this crate. The reader is
checked against samples written by ECMWF's ecCodes library (`tests/gen_grib.py` regenerates them), so its section
offsets and packing arithmetic agree with an independent implementation.

## What it reads

- Section 0 (indicator), section 1 (identification: centre and reference time), section 2 (skipped), section 3
  (grid definition template 3.0, latitude/longitude), section 4 (product definition template 4.0: parameter
  category and number, forecast time), section 5 (data representation template 5.0, simple packing), section 6
  (bitmap, or none), section 7 (data), and the `7777` end marker.
- Several messages in one file, read in order.
- Values unpacked as `(R + X × 2^E) / 10^D`. Points the bitmap marks as missing are NaN.

## What it writes

- One message per `Message`, with simple packing at 16 bits per value, a latitude/longitude grid, and a bitmap
  when any value is NaN.

## What it does not do

- Other grid templates (Gaussian, polar stereographic, Lambert and so on), product templates (ensembles, statistical
  processing, satellite) and packing templates (JPEG 2000, PNG, complex packing, spectral data) are reported as
  unsupported.
- Predefined bitmaps (indicator 254) are not read.
- Scanning modes other than the default are returned as the raw flags; values stay in the order of the file, and
  the caller interprets the orientation.
- Local-use sections are skipped, not interpreted.
