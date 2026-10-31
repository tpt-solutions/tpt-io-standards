# tpt-io-arxml

A reader for AUTOSAR XML (`.arxml`) files: the package hierarchy and the model
elements in each package.

## Spec access and licence

The ARXML schema is published by AUTOSAR under its own licence terms. No schema text,
example model or element definition from AUTOSAR is copied into this crate. The
element and package structure is read from the XML the tests supply.

## What it reads

- The schema release, taken from the default namespace (for example `r4.0`).
- `<AR-PACKAGES>` and nested `<AR-PACKAGE>`s, each with its `SHORT-NAME`, UUID and
  absolute path (`/Components/Fusion`).
- `<ELEMENTS>` of each package: the element's tag (its kind), short name and UUID.

## Validation

`ArxmlFile::validate` requires unique short names among sibling packages, and unique
element short names within each package.

## What it does not do

- It is read-only: there is no writer.
- Element contents, references, ECUC parameter values and multi-level element
  trees inside elements are not interpreted.
- `xsi:schemaLocation` and the schema XSD are not used to validate the document.
