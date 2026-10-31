# tpt-io-cellml

A read-only reader for CellML 1.x models, the XML format for mathematical models of biological and
physical systems.

## Spec access and licence

CellML is an open standard maintained by the CellML project. No specification text, model repository
entry, or sample model is copied into this crate; the test model is written for these tests.

## What it reads

- The model `name`.
- `units` definitions with their `unit` terms (units name and exponent, defaulting to 1).
- `component`s: name, each `variable` (name, `units`, `interface`), and the number of `math` blocks.
- `connection`s: the two component names from `map_components` and the variable pairs from
  `map_variables`.

## What it does not do

- Read only. Equations inside `math` are not parsed or evaluated.
- CellML 2.0 namespaces and metadata (RDF) are not read.
- Models are not checked for unit consistency or for ODE well-posedness.
