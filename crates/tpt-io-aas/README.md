# tpt-io-aas

A read-only reader for Asset Administration Shell (AAS) environments in the JSON
serialisation: shells, submodels, and the submodel elements they contain.

## Spec access and licence

The AAS metamodel and its serialisations are published by IDTA (the Industrial Digital Twin
Association) under their open terms. No specification text, schema or sample environment is
copied into this crate; the tests use environments written for them.

## What it reads

- `assetAdministrationShells`: each shell's `id`, `idShort`, `assetInformation.globalAssetId`,
  and the last key value of each submodel reference.
- `submodels`: each submodel's `id`, `idShort` and its `submodelElements` (`idShort` and
  `modelType`). `modelType` may be a string or an object with a `name`.
- `conceptDescriptions`: counted only.

## Validation

`Environment::validate` requires unique shell ids and unique submodel ids. References to
submodels outside the environment are allowed, since AAS references can be external.

## What it does not do

- Read only, and JSON only. The XML and V2 serialisations are not read.
- Element values, properties' value types, collection members and semantic references are
  not decoded.
- Concept descriptions are counted, not read.
