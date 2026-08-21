//! Interpret parsed JSON into the GeoJSON (RFC 7946) model.

use alloc::string::{String, ToString};
use alloc::vec::Vec;

use tpt_io_core::{ParseError, SpecReference};

use crate::error::GeoError;
use crate::model::{Bbox, Feature, FeatureCollection, GeoJson, Geometry, Position};
use tpt_io_core::json::{parse_json, Json};

fn geo_err(msg: impl ToString) -> GeoError {
    GeoError::Core(ParseError::custom(0, msg).with_spec(SpecReference::new("GeoJSON", "RFC 7946")))
}

fn require_object(j: &Json) -> Result<&alloc::collections::BTreeMap<String, Json>, GeoError> {
    match j {
        Json::Object(m) => Ok(m),
        _ => Err(geo_err("GeoJSON root must be a JSON object")),
    }
}

fn member<'a>(
    obj: &'a alloc::collections::BTreeMap<String, Json>,
    key: &str,
) -> Result<&'a Json, GeoError> {
    obj.get(key).ok_or_else(|| geo_err(concat_str("missing required member: ", key)))
}

// Small helper because `concat!` needs a literal; build the message manually.
fn concat_str(prefix: &'static str, key: &str) -> String {
    let mut s = String::from(prefix);
    s.push_str(key);
    s
}

fn read_position(j: &Json) -> Result<Position, GeoError> {
    let arr = match j {
        Json::Array(a) => a,
        _ => return Err(geo_err("position must be an array of numbers")),
    };
    if arr.len() < 2 {
        return Err(geo_err("position needs at least [lon, lat]"));
    }
    let lon = arr[0].as_f64().ok_or_else(|| geo_err("position longitude not a number"))?;
    let lat = arr[1].as_f64().ok_or_else(|| geo_err("position latitude not a number"))?;
    let elevation = arr.get(2).and_then(|v| v.as_f64());
    Ok(Position { lon, lat, elevation })
}

fn read_positions(j: &Json) -> Result<Vec<Position>, GeoError> {
    let arr = match j {
        Json::Array(a) => a,
        _ => return Err(geo_err("expected an array of positions")),
    };
    arr.iter().map(read_position).collect()
}

fn read_line_seq(j: &Json) -> Result<Vec<Vec<Position>>, GeoError> {
    let arr = match j {
        Json::Array(a) => a,
        _ => return Err(geo_err("expected an array of lines")),
    };
    arr.iter().map(read_positions).collect()
}

fn read_polygon_seq(j: &Json) -> Result<Vec<Vec<Position>>, GeoError> {
    read_line_seq(j)
}

fn read_polygon_seq_seq(j: &Json) -> Result<Vec<Vec<Vec<Position>>>, GeoError> {
    let arr = match j {
        Json::Array(a) => a,
        _ => return Err(geo_err("expected an array of polygons")),
    };
    arr.iter().map(read_polygon_seq).collect()
}

fn read_bbox(j: &Json) -> Result<Option<Bbox>, GeoError> {
    match j {
        Json::Array(a) => {
            let vals = a
                .iter()
                .map(|v| v.as_f64().ok_or_else(|| geo_err("bbox entries must be numbers")))
                .collect::<Result<Vec<f64>, GeoError>>()?;
            Ok(Some(Bbox(vals)))
        }
        Json::Null => Ok(None),
        _ => Err(geo_err("bbox must be an array")),
    }
}

fn read_geometry(j: &Json) -> Result<Geometry, GeoError> {
    let obj = require_object(j)?;
    let type_name = member(obj, "type")?
        .as_str()
        .ok_or_else(|| geo_err("geometry \"type\" must be a string"))?;
    Ok(match type_name {
        "Point" => Geometry::Point(read_position(member(obj, "coordinates")?)?),
        "MultiPoint" => Geometry::MultiPoint(read_positions(member(obj, "coordinates")?)?),
        "LineString" => Geometry::LineString(read_positions(member(obj, "coordinates")?)?),
        "MultiLineString" => Geometry::MultiLineString(read_line_seq(member(obj, "coordinates")?)?),
        "Polygon" => Geometry::Polygon(read_polygon_seq(member(obj, "coordinates")?)?),
        "MultiPolygon" => {
            Geometry::MultiPolygon(read_polygon_seq_seq(member(obj, "coordinates")?)?)
        }
        "GeometryCollection" => {
            let geoms = match member(obj, "geometries")? {
                Json::Array(a) => a,
                _ => return Err(geo_err("GeometryCollection needs \"geometries\"")),
            };
            Geometry::GeometryCollection(
                geoms.iter().map(read_geometry).collect::<Result<Vec<_>, GeoError>>()?,
            )
        }
        other => return Err(geo_err(concat_str("unknown geometry type: ", other))),
    })
}

fn read_geometry_opt(j: &Json) -> Result<Option<Geometry>, GeoError> {
    match j {
        Json::Null => Ok(None),
        other => Ok(Some(read_geometry(other)?)),
    }
}

fn read_feature(j: &Json) -> Result<Feature, GeoError> {
    let obj = require_object(j)?;
    if member(obj, "type")?.as_str() != Some("Feature") {
        return Err(geo_err("feature \"type\" must be \"Feature\""));
    }
    let geometry = read_geometry_opt(member(obj, "geometry")?)?;
    let properties = match member(obj, "properties")? {
        Json::Null => None,
        other => Some(other.clone()),
    };
    let id = obj.get("id").cloned();
    let bbox = match obj.get("bbox") {
        Some(b) => read_bbox(b)?,
        None => None,
    };
    Ok(Feature { geometry, properties, id, bbox })
}

fn read_feature_collection(j: &Json) -> Result<FeatureCollection, GeoError> {
    let obj = require_object(j)?;
    if member(obj, "type")?.as_str() != Some("FeatureCollection") {
        return Err(geo_err("\"type\" must be \"FeatureCollection\""));
    }
    let features = match member(obj, "features")? {
        Json::Array(a) => a,
        _ => return Err(geo_err("\"features\" must be an array")),
    };
    let features = features.iter().map(read_feature).collect::<Result<Vec<_>, GeoError>>()?;
    let bbox = match obj.get("bbox") {
        Some(b) => read_bbox(b)?,
        None => None,
    };
    Ok(FeatureCollection { features, bbox })
}

/// Parse a byte slice into a GeoJSON document.
///
/// The top-level form is dispatched on its `"type"` member: `FeatureCollection`
/// and `Feature` are handled directly, and any of the seven geometry type
/// strings are interpreted as a bare geometry.
pub fn parse_geojson(input: &[u8]) -> Result<GeoJson, GeoError> {
    let root = parse_json(input)?;
    let obj = require_object(&root)?;
    let type_name = member(obj, "type")?
        .as_str()
        .ok_or_else(|| geo_err("top-level \"type\" must be a string"))?;
    let doc = match type_name {
        "FeatureCollection" => GeoJson::FeatureCollection(read_feature_collection(&root)?),
        "Feature" => GeoJson::Feature(read_feature(&root)?),
        // Any geometry type name is a valid bare geometry.
        "Point" | "MultiPoint" | "LineString" | "MultiLineString" | "Polygon" | "MultiPolygon"
        | "GeometryCollection" => GeoJson::Geometry(read_geometry(&root)?),
        other => return Err(geo_err(concat_str("unsupported top-level type: ", other))),
    };
    Ok(doc)
}

/// Reader for GeoJSON documents, implementing [`tpt_io_core::Reader`].
#[derive(Debug, Clone, Default)]
pub struct GeoJsonReader {
    validation: tpt_io_core::ValidationSettings,
}

impl GeoJsonReader {
    /// Create a reader with default validation.
    pub fn new() -> Self {
        Self::default()
    }
    /// Set the validation configuration.
    pub fn with_validation(mut self, validation: tpt_io_core::ValidationSettings) -> Self {
        self.validation = validation;
        self
    }
}

impl tpt_io_core::Reader<GeoJson> for GeoJsonReader {
    type Error = GeoError;
    fn parse(&self, input: &[u8]) -> Result<GeoJson, GeoError> {
        let doc = parse_geojson(input)?;
        self.validation.run(&GeoJsonValidator, &doc)?;
        Ok(doc)
    }
}

/// Syntactic/semantic validation for a parsed [`GeoJson`] document.
#[derive(Debug, Clone, Copy, Default)]
pub struct GeoJsonValidator;

impl tpt_io_core::Validator<GeoJson> for GeoJsonValidator {
    type Error = GeoError;
    fn validate_syntactic(&self, value: &GeoJson) -> Result<(), GeoError> {
        if let GeoJson::FeatureCollection(fc) = value {
            for f in &fc.features {
                if f.geometry.is_none() && f.properties.is_none() {
                    return Err(geo_err("feature must have geometry or properties"));
                }
            }
        }
        Ok(())
    }
    fn validate_semantic(&self, _value: &GeoJson) -> Result<(), GeoError> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tpt_io_core::Reader;

    const FC: &str = r#"{
      "type": "FeatureCollection",
      "features": [
        {
          "type": "Feature",
          "geometry": { "type": "Point", "coordinates": [125.6, 10.1] },
          "properties": { "name": "Dinagat Islands" }
        },
        {
          "type": "Feature",
          "geometry": {
            "type": "Polygon",
            "coordinates": [[[0,0],[4,0],[4,4],[0,4],[0,0]]]
          },
          "properties": null
        }
      ]
    }"#;

    #[test]
    fn parses_feature_collection() {
        let doc = parse_geojson(FC.as_bytes()).unwrap();
        let fc = match doc {
            GeoJson::FeatureCollection(fc) => fc,
            _ => panic!("expected FeatureCollection"),
        };
        assert_eq!(fc.features.len(), 2);
        let pt = fc.features[0].geometry.as_ref().unwrap();
        assert_eq!(pt, &Geometry::Point(Position { lon: 125.6, lat: 10.1, elevation: None }));
        assert_eq!(
            fc.features[0].properties.as_ref().unwrap().get("name").and_then(|v| v.as_str()),
            Some("Dinagat Islands")
        );
        let poly = fc.features[1].geometry.as_ref().unwrap();
        match poly {
            Geometry::Polygon(rings) => {
                assert_eq!(rings.len(), 1);
                assert_eq!(rings[0].len(), 5);
            }
            _ => panic!("expected polygon"),
        }
    }

    #[test]
    fn parses_bare_geometry_and_feature() {
        let g = parse_geojson(br#"{"type":"LineString","coordinates":[[0,0],[1,1]]}"#).unwrap();
        assert!(matches!(g, GeoJson::Geometry(Geometry::LineString(_))));

        let f =
            parse_geojson(br#"{"type":"Feature","geometry":null,"properties":{"a":1}}"#).unwrap();
        match f {
            GeoJson::Feature(feat) => {
                assert!(feat.geometry.is_none());
                assert!(feat.properties.is_some());
            }
            _ => panic!("expected Feature"),
        }
    }

    #[test]
    fn rejects_unknown_type() {
        assert!(parse_geojson(br#"{"type":"Banana"}"#).is_err());
    }

    #[test]
    fn reader_runs_validation() {
        let r = GeoJsonReader::new();
        let doc = r.parse(FC.as_bytes()).unwrap();
        assert!(matches!(doc, GeoJson::FeatureCollection(_)));
    }
}
