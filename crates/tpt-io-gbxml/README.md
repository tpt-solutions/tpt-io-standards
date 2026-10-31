# tpt-io-gbxml

A read-only reader for gbXML, the XML format used to exchange building geometry and
energy-model data between CAD and building-energy tools.

## Spec access and licence

gbXML is published by the gbXML consortium under an open licence. No schema text or sample
model is copied into this crate; the test document is written for these tests.

## What it reads

- Every `Building` in the document (directly or inside a `Campus`): id, `buildingType`, `Name`.
- Each building's `Space` elements: id and optional name.
- Each building's `Surface` elements: id, `surfaceType`, and the `CartesianPoint` coordinates of
  every `PolyLoop` inside its `PlanarGeometry`.

## What it does not do

- Read only.
- Construction layers, opening and window detail, schedules, zones, and HVAC systems are not
  decoded.
- Surface and space areas are not computed.
