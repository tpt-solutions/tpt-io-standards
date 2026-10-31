# tpt-io-hecras

A read-only reader for HEC-RAS text geometry files (`.g01` and similar), the river-hydraulics model
format used by the US Army Corps of Engineers' HEC-RAS.

## Spec access and licence

The HEC-RAS text formats are documented by the US Army Corps of Engineers, whose documentation is public
domain. No text, model or dataset is copied into this crate; the test geometry is written for these tests.

## What it reads

- `River Reach=river,reach`: each reach's names.
- `Reach XY=` with its count: the reach centreline as `(x, y)` points.
- `Type RM … = 1 ,station,…`: a cross section at a river station.
- `#Sta/Elev=` with its count: the cross section's `(station, elevation)` points.

Numbers are read as whitespace- or comma-separated values, so the fixed-width layout is accepted when its
fields are separated by spaces.

## What it does not do

- Read only. Bridges, culverts, inline and lateral structures, storage areas, and the plan, flow and
  unsteady files are not read.
- Manning's n, bank stations, and other cross-section attributes are not decoded.
- Geometry is not validated against HEC-RAS's own consistency rules.
