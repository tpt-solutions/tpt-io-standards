//! Reading and writing WaterML 2.0 measurement time series.

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::fmt::Write as _;

use tpt_io_core::xml::{self, escape_attr, escape_text, Element};
use tpt_io_core::ParseError;

use crate::error::WaterMlError;

/// One time-value pair.
#[derive(Debug, Clone, PartialEq)]
pub struct Point {
    /// Time as written in the document, usually ISO 8601.
    pub time: String,
    /// The measured value.
    pub value: f64,
}

/// A measurement time series.
#[derive(Debug, Clone, PartialEq)]
pub struct TimeSeries {
    /// The series identifier (`gml:id`).
    pub id: String,
    /// The observed property URI or name (`om:observedProperty`).
    pub observed_property: String,
    /// The unit of measure code (`wml2:uom`), e.g. `m3/s`.
    pub uom: String,
    /// The time-value pairs in document order.
    pub points: Vec<Point>,
}

/// Read every measurement time series in a WaterML 2.0 document.
///
/// Each series is read with the `observedProperty` of its enclosing `OM_Observation`, since the
/// standard places the property beside the result rather than inside it.
pub fn parse(input: &[u8]) -> Result<Vec<TimeSeries>, WaterMlError> {
    let root = xml::parse(input)?;
    let mut out = Vec::new();
    for obs in root.descendants_named("OM_Observation") {
        let property = obs
            .descendants_named("observedProperty")
            .first()
            .and_then(|p| p.attr("href"))
            .unwrap_or("");
        for ts in timeseries_in(obs) {
            out.push(series(ts, property)?);
        }
    }
    if out.is_empty() {
        // A bare series with no observation wrapper.
        for ts in timeseries_in(&root) {
            out.push(series(ts, "")?);
        }
    }
    Ok(out)
}

/// Every measurement time series under `el`. Some producers spell the element `…TimeSeries`.
fn timeseries_in(el: &Element) -> Vec<&Element> {
    let mut v = el.descendants_named("MeasurementTimeseries");
    v.extend(el.descendants_named("MeasurementTimeSeries"));
    v
}

fn series(el: &Element, observed_property: &str) -> Result<TimeSeries, WaterMlError> {
    let id = el
        .attr("id")
        .ok_or_else(|| ParseError::custom(el.offset, "time series has no gml:id"))?
        .to_string();
    let observed_property = observed_property.to_string();
    let uom = el
        .descendants_named("uom")
        .first()
        .and_then(|u| u.attr("code"))
        .unwrap_or("")
        .to_string();
    let mut points = Vec::new();
    for p in el.children_named("point") {
        let tvp = p
            .child("MeasurementTVP")
            .ok_or_else(|| WaterMlError::Invalid("point without MeasurementTVP".to_string()))?;
        let time = tvp
            .child("time")
            .map(|t| t.text_trimmed().to_string())
            .ok_or_else(|| WaterMlError::Invalid("MeasurementTVP without time".to_string()))?;
        let raw = tvp
            .child("value")
            .map(|v| v.text_trimmed())
            .ok_or_else(|| WaterMlError::Invalid("MeasurementTVP without value".to_string()))?;
        let value = raw
            .parse::<f64>()
            .map_err(|_| ParseError::custom(tvp.offset, "value is not a number"))?;
        points.push(Point { time, value });
    }
    Ok(TimeSeries { id, observed_property, uom, points })
}

/// Write time series as a WaterML 2.0 collection. `parse(write(s))` gives back the same series.
pub fn write(series: &[TimeSeries]) -> Vec<u8> {
    let mut out = String::new();
    out.push_str(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <wml2:Collection xmlns:wml2=\"http://www.opengis.net/waterml/2.0\" \
         xmlns:om=\"http://www.opengis.net/om/2.0\" \
         xmlns:gml=\"http://www.opengis.net/gml/3.2\" \
         xmlns:xlink=\"http://www.w3.org/1999/xlink\">\n",
    );
    for ts in series {
        out.push_str("  <wml2:observationMember>\n    <om:OM_Observation>\n");
        let _ = writeln!(
            out,
            "      <om:observedProperty xlink:href=\"{}\"/>",
            escape_attr(&ts.observed_property)
        );
        out.push_str("      <om:result>\n");
        let _ = writeln!(out, "        <wml2:MeasurementTimeseries gml:id=\"{}\">", escape_attr(&ts.id));
        let _ = writeln!(
            out,
            "          <wml2:defaultPointMetadata><wml2:DefaultTVPMeasurementMetadata>\
             <wml2:uom code=\"{}\"/></wml2:DefaultTVPMeasurementMetadata></wml2:defaultPointMetadata>",
            escape_attr(&ts.uom)
        );
        for p in &ts.points {
            let _ = writeln!(
                out,
                "          <wml2:point><wml2:MeasurementTVP><wml2:time>{}</wml2:time>\
                 <wml2:value>{}</wml2:value></wml2:MeasurementTVP></wml2:point>",
                escape_text(&p.time),
                p.value
            );
        }
        out.push_str("        </wml2:MeasurementTimeseries>\n      </om:result>\n    </om:OM_Observation>\n  </wml2:observationMember>\n");
    }
    out.push_str("</wml2:Collection>\n");
    out.into_bytes()
}
