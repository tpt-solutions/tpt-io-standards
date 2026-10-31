# tpt-io-witsml

A read-only reader for WITSML well trajectories, the oil-and-gas XML standard for drilling and
well data exchange.

## Spec access and licence

WITSML is published by Energistics under its licence terms. No schema text or sample survey is
copied into this crate; the test document is written for these tests.

## What it reads

- Each `trajectory`: `uid`, `uidWell`, `uidWellbore`, and its `name`.
- Each `trajectoryStation`: `uid`, measured depth (`md`) with its `uom`, inclination (`incl`) and
  azimuth (`azi`), as numbers in the units written.

A station without an `md`, `incl` or `azi` is rejected as invalid.

## What it does not do

- Read only.
- Units are not converted; `md` stays in the unit given by `uom`.
- Corrections, error models and survey tool metadata are not read.
- WITSML 2.0 and RESQML are not read.
