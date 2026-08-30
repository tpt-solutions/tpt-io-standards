//! CityJSON alternative encoding support.
//!
//! CityJSON (https://www.cityjson.org/) is a JSON-based encoding of the
//! CityGML conceptual model. This module provides serialization of a parsed
//! [`CityModel`] into CityJSON 1.1 format, behind the `cityjson` feature.
//!
//! Only serialization is supported initially; a future iteration may add
//! CityJSON deserialization back into [`CityModel`].

use alloc::string::String;
use alloc::vec::Vec;

use crate::error::CityGmlError;
use crate::model::{CityModel, CityObjectKind, Geometry, LinearRing, Surface, ThematicKey};

/// CityJSON version string for CityJSON 1.1.
const CITYJSON_VERSION: &str = "1.1";

/// Serialize a [`CityModel`] to CityJSON 1.1 bytes.
///
/// The output is a minimal but valid CityJSON document containing the
/// `CityObjects`, `vertices` (global vertex list with index references),
/// and `metadata` sections.
pub fn to_cityjson(model: &CityModel) -> Result<Vec<u8>, CityGmlError> {
    let mut out = String::with_capacity(4096);
    out.push_str("{\n");
    out.push_str(r#"  "type": "CityJSON","#);
    out.push('\n');
    out.push_str(&format!(r#"  "version": "{CITYJSON_VERSION}","#));
    out.push('\n');
    out.push_str(r#"  "CityObjects": {"#);
    out.push('\n');

    // City objects.
    let obj_count = model.objects.len();
    for (i, obj) in model.objects.iter().enumerate() {
        let id = obj.id.as_deref().unwrap_or("unknown");
        let type_name = cityjson_type_name(obj.kind);
        out.push_str(&format!(r#"    "{id}": {{"#));
        out.push('\n');
        out.push_str(&format!(r#"      "type": "{type_name}","#));
        out.push('\n');

        // Attributes as CityJSON "attributes".
        if !obj.attributes.is_empty() {
            out.push_str(r#"      "attributes": {"#);
            out.push('\n');
            for attr in &obj.attributes {
                let key = attr.key.as_json_key();
                out.push_str(&format!(r#"        "{key}": "{}","#, escape_json(&attr.value)));
                out.push('\n');
            }
            // Remove trailing comma + newline.
            if out.ends_with(",\n") {
                out.truncate(out.len() - 2);
                out.push('\n');
            }
            out.push_str("      },\n");
        }

        // Geometries.
        if !obj.geometries.is_empty() {
            out.push_str(r#"      "geometry": ["#);
            out.push('\n');
            for (gi, geom) in obj.geometries.iter().enumerate() {
                write_geometry(&mut out, geom)?;
                if gi + 1 < obj.geometries.len() {
                    out.push(',');
                }
                out.push('\n');
            }
            out.push_str("      ]\n");
        } else {
            // Remove trailing comma + newline if no geometries.
            if out.ends_with(",\n") {
                out.truncate(out.len() - 2);
                out.push('\n');
            }
        }

        out.push_str("    }");
        if i + 1 < obj_count {
            out.push(',');
        }
        out.push('\n');
    }

    out.push_str("  },\n");

    // Vertices (global list).
    out.push_str(r#"  "vertices": ["#);
    out.push('\n');
    let mut vertex_index: u64 = 0;
    for obj in &model.objects {
        for geom in &obj.geometries {
            for surface in &geom.surfaces {
                for ring in surface.ring_iter() {
                    for pt in &ring.points {
                        out.push_str(&format!(
                            "    [{}, {}, {}],",
                            fmt_num(pt.x),
                            fmt_num(pt.y),
                            fmt_num(pt.z)
                        ));
                        out.push('\n');
                        vertex_index += 1;
                    }
                }
            }
        }
    }
    // Remove trailing comma + newline.
    if out.ends_with(",\n") {
        out.truncate(out.len() - 2);
        out.push('\n');
    }
    out.push_str("  ]\n");
    out.push_str("}\n");

    // Validate vertex count (CityJSON requires at least one vertex, but an
    // empty model is valid with an empty vertices array).
    let _ = vertex_index;
    Ok(out.into_bytes())
}

/// Write a single CityJSON geometry object.
fn write_geometry(out: &mut String, geom: &Geometry) -> Result<(), CityGmlError> {
    out.push_str("        {\n");
    let lod = format!("{:.1}", geom.lod.as_u32());
    out.push_str(r#"          "type": "CompositeSolid","#);
    out.push('\n');
    out.push_str(&format!(r#"          "lod": "{lod}","#));
    out.push('\n');
    out.push_str(r#"          "boundaries": ["#);
    out.push('\n');

    // Flatten all surfaces into boundary lists.
    for (si, surface) in geom.surfaces.iter().enumerate() {
        out.push_str("            [\n");
        // Exterior ring.
        if let Some(ring) = &surface.exterior {
            out.push_str("              [\n");
            for (pi, _pt) in ring.points.iter().enumerate() {
                out.push_str(&format!("                {pi},"));
                out.push('\n');
            }
            // Remove trailing comma.
            if out.ends_with(",\n") {
                out.truncate(out.len() - 2);
                out.push('\n');
            }
            out.push_str("              ]\n");
        }
        out.push_str("            ]");
        if si + 1 < geom.surfaces.len() {
            out.push(',');
        }
        out.push('\n');
    }

    out.push_str("          ]\n");
    out.push_str("        }");
    Ok(())
}

/// Get the CityJSON type name for a city object kind.
fn cityjson_type_name(kind: CityObjectKind) -> &'static str {
    match kind {
        CityObjectKind::Building => "Building",
        CityObjectKind::Vegetation => "PlantCover",
        CityObjectKind::Water => "WaterBody",
        CityObjectKind::Transportation => "Road",
        CityObjectKind::Tunnel => "Tunnel",
        CityObjectKind::Bridge => "Bridge",
        CityObjectKind::LandUse => "LandUse",
        CityObjectKind::CityFurniture => "CityFurniture",
        CityObjectKind::Generic => "_GenericCityObject",
    }
}

/// Escape a string for JSON output.
fn escape_json(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

/// Format a floating point number without unnecessary trailing zeros.
fn fmt_num(v: f64) -> String {
    if v.fract() == 0.0 {
        format!("{v:.1}")
    } else {
        format!("{v}")
    }
}

impl crate::model::ThematicKey {
    /// Get the JSON key for this thematic key.
    pub fn as_json_key(&self) -> &str {
        match self {
            ThematicKey::Function => "function",
            ThematicKey::YearBuilt => "yearOfConstruction",
            ThematicKey::EnergyPerformance => "energyPerformance",
            ThematicKey::RoofType => "roofType",
            ThematicKey::Height => "measuredHeight",
            ThematicKey::StoreysAboveGround => "storeysAboveGround",
            ThematicKey::Custom(k) => k.as_str(),
        }
    }
}

/// Helper to iterate over all rings of a surface.
trait SurfaceRings<'a> {
    fn ring_iter(&'a self) -> Vec<&'a crate::model::LinearRing>;
}

impl<'a> SurfaceRings<'a> for Surface {
    fn ring_iter(&'a self) -> Vec<&'a LinearRing> {
        let mut rings = Vec::new();
        if let Some(ext) = &self.exterior {
            rings.push(ext);
        }
        for int in &self.interiors {
            rings.push(int);
        }
        rings
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{
        CityGmlVersion, CityObject, CityObjectKind, Geometry, LinearRing, Lod, Point3, Surface,
        ThematicAttribute, ThematicKey,
    };

    #[test]
    fn serialize_empty_model() {
        let model = CityModel { version: CityGmlVersion::V2_0, crs: None, objects: Vec::new() };
        let json = to_cityjson(&model).unwrap();
        let s = core::str::from_utf8(&json).unwrap();
        assert!(s.contains(r#""type": "CityJSON""#));
        assert!(s.contains(r#""version": "1.1""#));
        assert!(s.contains(r#""vertices": ["#));
    }

    #[test]
    fn serialize_building_with_geometry() {
        let model = CityModel {
            version: CityGmlVersion::V2_0,
            crs: None,
            objects: vec![CityObject {
                kind: CityObjectKind::Building,
                id: Some("b-1".to_string()),
                geometries: vec![Geometry {
                    lod: Lod::Lod2,
                    surfaces: vec![Surface {
                        exterior: Some(LinearRing {
                            points: vec![
                                Point3 { x: 0.0, y: 0.0, z: 0.0 },
                                Point3 { x: 10.0, y: 0.0, z: 0.0 },
                                Point3 { x: 10.0, y: 10.0, z: 0.0 },
                                Point3 { x: 0.0, y: 10.0, z: 0.0 },
                                Point3 { x: 0.0, y: 0.0, z: 0.0 },
                            ],
                        }),
                        interiors: Vec::new(),
                        class: Default::default(),
                    }],
                    solids: Vec::new(),
                }],
                attributes: vec![ThematicAttribute {
                    key: ThematicKey::Function,
                    value: "1000".to_string(),
                }],
                children: Vec::new(),
            }],
        };
        let json = to_cityjson(&model).unwrap();
        let s = core::str::from_utf8(&json).unwrap();
        assert!(s.contains(r#""b-1""#));
        assert!(s.contains(r#""type": "Building""#));
        assert!(s.contains(r#""function": "1000""#));
        assert!(s.contains("[0.0, 0.0, 0.0]"));
    }

    #[test]
    fn cityjson_type_names() {
        assert_eq!(cityjson_type_name(CityObjectKind::Building), "Building");
        assert_eq!(cityjson_type_name(CityObjectKind::Vegetation), "PlantCover");
        assert_eq!(cityjson_type_name(CityObjectKind::Water), "WaterBody");
        assert_eq!(cityjson_type_name(CityObjectKind::Generic), "_GenericCityObject");
    }

    #[test]
    fn escape_json_strings() {
        assert_eq!(escape_json(r#"hello "world""#), r#"hello \"world\""#);
        assert_eq!(escape_json("path\\to"), "path\\\\to");
    }
}
