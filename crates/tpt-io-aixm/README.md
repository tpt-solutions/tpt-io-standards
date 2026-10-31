# tpt-io-aixm

A read-only reader for AIXM 5.1 aeronautical data: airports and heliports, navigation aids,
airspaces and routes, from AIXM XML messages.

## Spec access and licence

AIXM is published by EUROCONTROL and the FAA under their open terms. No schema text, data
set or sample message is copied into this crate; the test messages are written for these tests.

## What it reads

For each `hasMember` element, the feature it contains:

- its kind (`AirportHeliport`, `Navaid`, `Airspace`, `Route`, or the element name);
- its `gml:id`;
- the `designator` and `name` from its first time slice;
- every `gml:pos` (one latitude/longitude pair) and `gml:posList` (pairs), in order.

## What it does not do

- Read only.
- Only the first time slice is used. Later slices, validity periods and history are not read.
- Service, restriction, runway, and airspace-volume detail are not decoded.
- Coordinates are kept in the order written (latitude then longitude); no CRS conversion.
