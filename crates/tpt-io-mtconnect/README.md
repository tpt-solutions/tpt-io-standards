# tpt-io-mtconnect

A read-only reader for MTConnect XML: the device model from `MTConnectDevices` and the
observations from `MTConnectStreams`.

## Spec access and licence

MTConnect is published by the MTConnect Institute under its open licence. No standard text,
schema or sample document is copied into this crate; the test documents are written for these
tests.

## What it reads

**Devices** (`parse_devices`):
- `schemaVersion` from the header.
- Each `Device` (id, UUID, name) and its component tree: every component's id, element kind
  (`Axes`, `Controller`, `Linear` and so on), name, data items and nested components.
- Each `DataItem`: id, name, category (`SAMPLE`, `EVENT`, `CONDITION`), type, and subtype.

**Streams** (`parse_streams`):
- Every `DeviceStream` UUID, with each `ComponentStream`'s `Samples`, `Events` and
  `Condition` children as observations: data item id, element kind, timestamp, sequence and
  value text.

## Validation

- `Devices::validate` requires data item ids to be unique across the document.
- `Streams::validate_against` requires every observation to refer to a data item in the devices
  document.

## What it does not do

- Read only: there is no writer.
- Timestamps are kept as text; they are not converted to a time type.
- `Condition` detail (level, native code, text) is not decoded beyond the element's value.
- Assets, `Message`/`Asset` streams and the `Header` buffer attributes are not modelled.
