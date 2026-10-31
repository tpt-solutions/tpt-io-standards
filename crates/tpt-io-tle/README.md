# tpt-io-tle

A reader and writer for two-line element sets (TLE), the fixed-width orbital element
format used to publish satellite positions, with checksum verification.

## Spec access and licence

The TLE format is documented by NORAD and in the public CelesTrak format notes. No text
from those sources is copied into this crate; the field positions are implemented from
the column layout. Test sets are made up for these tests.

## What it reads

- Optional name line, then the two 69-column lines, repeated for each set in a file.
- Every field in the two lines: satellite number, classification, international
  designator, epoch (two-digit year mapped to 1957–2056), day of year, mean-motion
  derivatives, B\*, ephemeris type, element set, and the line 2 elements.
- Both checksums. A mismatch is reported as `TleError::Checksum` with the satellite
  number and line.
- Satellite numbers must agree across the two lines.

## Validation

Eccentricity in `[0, 1)`, inclination in `[0, 180]` degrees, positive mean motion, and
an epoch day in `[1, 367)`.

## Writing

`write` emits canonical column widths and recomputes checksums. A set that came from
a canonically formatted TLE round-trips exactly. A value with more precision than the
columns hold is rounded on output, so `parse(write(x)) == x` holds only for values at
TLE precision.

## Example

```rust
let sets = tpt_io_tle::parse(&std::fs::read("active.tle")?)?;
for s in &sets {
    println!("{:?} {} inc={:.4}", s.name, s.satellite_number, s.inclination);
}
```
