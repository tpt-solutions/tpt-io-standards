# tpt-io-landxml

A reader and writer for LandXML, the XML exchange format for civil engineering and surveying data
(terrain surfaces, alignments, and survey points).

## Spec access and licence

LandXML is published by the LandXML consortium under an open licence. No schema text or sample
project is copied into this crate; the test document is written for these tests.

## What it reads

- The linear unit from the `Units` element (`Metric` or `Imperial`, its `linearUnit` attribute).
- The `Project` name.
- Each `Surface` in `Surfaces`: its name, the `P` points with their ids and `x y z`, and the `F` faces
  as three point ids.
- Alignment names from `Alignments`.

## Validation

`LandXml::validate` requires unique point ids within each surface, faces that refer to existing
points, and faces with three distinct points.

## What it writes

Units, project, surfaces (points and faces) and alignment names. Parsing the output gives back the same
document.

## What it does not do

- Alignment geometry (stations, curves, spirals) is not read.
- Cogo points, pipe networks, and feature dictionaries are ignored.
- Coordinates are kept as written, with no unit conversion.
