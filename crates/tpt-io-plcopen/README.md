# tpt-io-plcopen

A read-only reader for PLCopen XML (the TC6 exchange format for IEC 61131-3 programs),
as exported by most engineering tools for programmable logic controllers.

## Spec access and licence

The PLCopen XML schema is published by PLCopen under its own terms. No schema text, sample
project or language definition is copied into this crate; the tests use projects written
for them.

## What it reads

- `fileHeader/@productName` and `contentHeader/@name`.
- Data type names under `types/dataTypes`.
- Each `pou` with its name, kind (`program`, `functionBlock`, `function`, or the raw type
  string), and interface variables (section, name, type, `simpleValue` initial value).
  Derived types are reported by their `derived/@name`.
- The body: language, plus the source text for Structured Text and Instruction List. Text
  comes from the `xhtml` child, or directly from the language element.

## What it does not do

- Read only: there is no writer.
- FBD, LD and SFC networks are recognised but not decoded.
- Global variables, configurations, resources and tasks are not read.
- Type-level detail (array bounds, struct members, string lengths) is not decoded.
