# tpt-io-naif

Reader for NAIF DAF and SPK kernels: the binary ephemeris files of the SPICE toolkit (`.bsp`). An SPK kernel gives
the position and velocity of solar-system bodies over time.

## Spec access and licence

The DAF and SPK formats are documented in NAIF's SPICE required reading, which NAIF publishes openly. No
specification text is copied into this crate; the test kernel is built in code.

## What it reads

- The DAF file record: identifier, the double and integer counts per summary, the forward pointer, and the binary
  format (`LTL-IEEE` or `BIG-IEEE`).
- The summary records, following the forward-pointer chain, and the name record that follows each one.
- Segment words, from the summary's begin and end word addresses.
- SPK segments of data type 2 (Chebyshev position; velocity by differentiating the polynomials) and type 3
  (Chebyshev position and velocity, from separate coefficient sets).

## What it does not do

- Other SPK data types (1, 5, 8–21) and other DAF kernel families (PCK, CK, EK) are not evaluated; summaries and
  raw segment words are still available for them.
- Non-IEEE binary formats (`VAX-GFLT`, `VAX-DFLT`) are rejected.
- Time conversion (ephemeris time from calendar dates) is left to the caller.
