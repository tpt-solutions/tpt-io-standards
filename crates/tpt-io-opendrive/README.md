# tpt-io-opendrive

A reader, writer and validator for ASAM OpenDRIVE road networks (`.xodr`).

## Spec access and licence

ASAM OpenDRIVE is an ASAM standard. Its published document is subject to the
ASAM licence terms, so no specification text, schema or sample map is copied into
this crate. The element and attribute names used here follow the published
structure of the plan view and lane sections; test maps are written for these tests.

## What it reads and writes

- `<header>`: revision numbers and map name.
- `<road>`: id, name, length, and junction (`-1` becomes `None`).
- `<planView>` geometry: `line`, `arc`, `spiral`, `poly3` and `paramPoly3`.
- `<lanes>`: lane sections with `left`, `center` and `right` lanes, their types,
  levels and width records.

The writer produces the same structure, so parsing its output gives back an equal
`OpenDrive`.

## Validation

`OpenDrive::validate` checks unique road ids, geometry segments that join end to
start (within 1 mm), a first segment and lane section starting at `s = 0`,
increasing lane-section `s`, and unique lane ids within each section.

## What it does not do

- Objects, signals, junction connections, elevation and superelevation profiles,
  lane links and road-mark data are not modelled and are dropped.
- Geometry shapes other than the five listed above are rejected.
- Lane-width `poly3` and other profile data are not evaluated.
