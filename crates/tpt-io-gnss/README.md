# tpt-io-gnss

Read-only readers for GNSS data: RINEX 3 observation files, and SP3 precise orbit files.

## Spec access and licence

RINEX is maintained by the RINEX working group and the International GNSS Service (IGS); SP3 is an IGS
format. Both are published openly. No specification text, observation file or orbit product is copied into
this crate; the tests use short files written for them.

## What it reads

**RINEX 3 observations** (`parse_rinex_obs`):
- The version, `MARKER NAME` and `APPROX POSITION XYZ` from the header.
- `SYS / # / OBS TYPES`: observation codes per system, including continuation lines.
- Epochs (`>` lines): year, month, day, hour, minute, second, and the epoch flag.
- Satellite lines: the satellite id and its observation values. Each value is 14 characters
  (blank means missing); loss-of-lock and signal-strength flags are ignored.

**SP3** (`parse_sp3`):
- The version letter from the first line.
- Each epoch (`*` line) and its `P` position records: satellite id, `x y z` in kilometres, and the clock
  offset in microseconds (the `999999` missing value becomes `None`).

## What it does not do

- Read only. RINEX navigation files, meteorological files, and SP3 velocity and correlation records are not
  read.
- Satellite positions are not interpolated to other times and are not transformed between frames.
- The RINEX 2 layout is not read.
