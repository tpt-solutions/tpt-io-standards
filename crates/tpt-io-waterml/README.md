# tpt-io-waterml

A reader and writer for OGC WaterML 2.0 measurement time series, the XML format used
for hydrological observations such as river discharge and groundwater levels.

## Spec access and licence

WaterML 2.0 is an Open Geospatial Consortium standard, available under the OGC licence
terms. No specification text, schema or example document is copied into this crate;
the test documents are written for these tests.

## What it reads

- Every `MeasurementTimeseries` (or `MeasurementTimeSeries`) in the document, with:
  - its `gml:id`;
  - the `href` of its `om:observedProperty`;
  - the `code` of the first `uom` (unit of measure);
  - every `point`'s `MeasurementTVP` `time` and `value`.
- Namespace prefixes are ignored, so documents using other prefixes read the same way.

## What it writes

A `wml2:Collection` with one `observationMember` per series. Parsing the output gives
back the same series, including identifiers that contain characters needing escapes.

## What it does not do

- Quality flags, comments, metadata beyond the unit, and the feature of interest are not
  modelled and are dropped on read.
- Time values are kept as text; time zones and intervals are not converted.
- Only the first `uom` in each series is used.
