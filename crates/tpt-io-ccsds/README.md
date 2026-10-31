# tpt-io-ccsds

Readers for CCSDS orbit data messages in KVN (key = value) form.

| Message | Purpose | Read | Write |
| --- | --- | --- | --- |
| OEM | Orbit ephemeris: segments of state vectors | yes | yes |
| OPM | Orbit parameters: one state vector and metadata | yes | no |
| TDM | Tracking data: metadata and data blocks | yes | no |

## Spec access and licence

The messages are defined by the Consultative Committee for Space Data Systems (CCSDS)
recommendations, which are published under the CCSDS open standards terms. No recommendation
text is copied here. Test messages are written for these tests.

## What it reads

- **OEM**: `CCSDS_OEM_VERS`, `CREATION_DATE`, `ORIGINATOR`; each `META_START … META_STOP`
  block (object, centre, frame, time system, start and stop times, and any other keys);
  state rows `epoch x y z vx vy vz` (an optional acceleration triple is ignored);
  `COVARIANCE_START … COVARIANCE_STOP` blocks are skipped.
- **OPM**: all key/value pairs, plus a `StateVector` when `EPOCH`, `X`, `Y`, `Z`,
  `X_DOT`, `Y_DOT` and `Z_DOT` are all present.
- **TDM**: header pairs, then `META_START` and `DATA_START` blocks as ordered key/value lists.

Positions are in kilometres and velocities in kilometres per second. Epochs are kept as
written, not converted.

## What it does not do

- OPM Keplerian and covariance sections are kept only as raw pairs.
- TDM observations are not grouped into records; each block is a flat list.
- The writer covers OEM only.
- Frame transformations and time-system conversions are not performed.
