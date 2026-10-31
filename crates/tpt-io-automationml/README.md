# tpt-io-automationml

A read-only reader for AutomationML files (CAEX documents, IEC 62714), the XML exchange format
for plant engineering data across tools.

## Spec access and licence

AutomationML is defined by IEC 62714 and the AutomationML consortium. No standard text, class
library or sample plant is copied into this crate; the test file is written for these tests.

## What it reads

- Every top-level `InternalElement` of each `InstanceHierarchy`, recursively: its `Name`, `ID`,
  `RefBaseSystemUnitPath` (kept as the class path), its `Attribute`s with `AttributeDataType` and the
  `Value` text, and its nested `InternalElement`s.
- Every `InternalLink` with its name and both side references (`element:side`).

## What it does not do

- Read only.
- Role, interface and system-unit class libraries are not loaded, so class paths are kept as text.
- Attribute values are kept as text; no conversion by data type is done.
- Document-level `SuperiorStandardVersion` checks and XSD validation are not performed.
