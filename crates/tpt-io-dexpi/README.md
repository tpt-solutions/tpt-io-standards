# tpt-io-dexpi

A read-only reader for DEXPI P&ID exchange files (XML), the open format for process and
instrumentation diagrams.

## Spec access and licence

DEXPI is published by the DEXPI consortium under its licence terms. No schema text, class library or
sample diagram is copied into this crate. The element and attribute names follow the published DEXPI
model. **Only the names exercised by the test document are verified**; other DEXPI versions may use
names that this reader does not collect.

## What it reads

Every element that carries both an `ID` and a `ComponentClass` attribute, anywhere in the document:
its element name (`Equipment`, `PipingNetworkSegment`, `ProcessInstrumentationFunction`, and so on),
`ID`, `ComponentClass`, `ComponentName` (tag), and the `ID`s of its `Nozzle` descendants.

## What it does not do

- Read only, and no DEXPI class hierarchy is applied.
- Connections between nozzles and piping, geometry, and line numbering are not decoded.
- Documents are not validated against the DEXPI schema.
