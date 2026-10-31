//! Integration tests for the WaterML 2.0 reader and writer.

use tpt_io_waterml::{parse, write, Point, TimeSeries, WaterMlError};

const DOC: &[u8] = br#"<?xml version="1.0" encoding="UTF-8"?>
<wml2:Collection xmlns:wml2="http://www.opengis.net/waterml/2.0"
    xmlns:om="http://www.opengis.net/om/2.0" xmlns:gml="http://www.opengis.net/gml/3.2"
    xmlns:xlink="http://www.w3.org/1999/xlink">
  <wml2:observationMember>
    <om:OM_Observation>
      <om:observedProperty xlink:href="http://example.org/def/discharge"/>
      <om:result>
        <wml2:MeasurementTimeseries gml:id="station-42">
          <wml2:defaultPointMetadata>
            <wml2:DefaultTVPMeasurementMetadata>
              <wml2:uom code="m3/s"/>
            </wml2:DefaultTVPMeasurementMetadata>
          </wml2:defaultPointMetadata>
          <wml2:point>
            <wml2:MeasurementTVP>
              <wml2:time>2024-01-01T00:00:00Z</wml2:time>
              <wml2:value>12.5</wml2:value>
            </wml2:MeasurementTVP>
          </wml2:point>
          <wml2:point>
            <wml2:MeasurementTVP>
              <wml2:time>2024-01-01T01:00:00Z</wml2:time>
              <wml2:value>-0.25</wml2:value>
            </wml2:MeasurementTVP>
          </wml2:point>
        </wml2:MeasurementTimeseries>
      </om:result>
    </om:OM_Observation>
  </wml2:observationMember>
</wml2:Collection>"#;

#[test]
fn reads_series_units_and_points() {
    let series = parse(DOC).unwrap();
    assert_eq!(series.len(), 1);
    let s = &series[0];
    assert_eq!(s.id, "station-42");
    assert_eq!(s.observed_property, "http://example.org/def/discharge");
    assert_eq!(s.uom, "m3/s");
    assert_eq!(s.points.len(), 2);
    assert_eq!(s.points[1], Point { time: "2024-01-01T01:00:00Z".into(), value: -0.25 });
}

#[test]
fn write_then_parse_is_identity() {
    let series = parse(DOC).unwrap();
    let back = parse(&write(&series)).unwrap();
    assert_eq!(back, series);
}

#[test]
fn several_series_survive_a_round_trip() {
    let a = TimeSeries {
        id: "a".into(),
        observed_property: "p".into(),
        uom: "m".into(),
        points: vec![Point { time: "t0".into(), value: 1.0 }, Point { time: "t1".into(), value: 2.5 }],
    };
    let b = TimeSeries { id: "b & <c>".into(), observed_property: "q".into(), uom: "mm".into(), points: vec![] };
    assert_eq!(parse(&write(&[a.clone(), b.clone()])).unwrap(), vec![a, b]);
}

#[test]
fn documents_without_series_read_as_empty() {
    assert!(parse(b"<wml2:Collection xmlns:wml2=\"x\"/>").unwrap().is_empty());
}

#[test]
fn malformed_points_are_errors() {
    let no_value = br#"<x><MeasurementTimeseries id="s"><point><MeasurementTVP><time>t</time></MeasurementTVP></point></MeasurementTimeseries></x>"#;
    assert!(matches!(parse(no_value), Err(WaterMlError::Invalid(_))));
    let bad_value = br#"<x><MeasurementTimeseries id="s"><point><MeasurementTVP><time>t</time><value>ten</value></MeasurementTVP></point></MeasurementTimeseries></x>"#;
    assert!(parse(bad_value).is_err());
    assert!(parse(b"<wml2:Collection").is_err());
}
