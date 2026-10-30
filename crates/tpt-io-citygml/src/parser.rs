//! CityGML 2.0 / 3.0 XML parser.
//!
//! CityGML documents are XML-based with a well-defined namespace structure.
//! The parser uses [`quick-xml`] for streaming event-based parsing and builds
//! a structured [`CityModel`]. Both CityGML 2.0 (`http://www.opengis.net/citygml/2.0`)
//! and CityGML 3.0 (`http://www.opengis.net/citygml/3.0`) are supported.
//!
//! Parsing is driven by element names (stripped of namespace prefixes) so that
//! both schema versions can be handled by a single parser with version detection.

use alloc::string::{String, ToString};
use alloc::vec::Vec;

use quick_xml::events::Event;
use quick_xml::name::ResolveResult;
use quick_xml::NsReader;
use tpt_io_core::{Encoding, Reader, ValidationSettings, Validator};

use crate::error::CityGmlError;
use crate::model::{
    CityGmlVersion, CityModel, CityObject, CityObjectKind, Crs, Geometry, LinearRing, Lod, Point3,
    Solid, Surface, SurfaceClass, ThematicAttribute, ThematicKey,
};

/// CityGML 3.0 core namespace bytes.
const NS_CITYGML_3: &[u8] = b"http://www.opengis.net/citygml/3.0";

/// Parse a CityGML document from a string slice.
pub fn parse_str(input: &str) -> Result<CityModel, CityGmlError> {
    let mut reader = NsReader::from_str(input);
    reader.config_mut().trim_text(true);
    let mut parser = XmlParser::default();
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) | Ok(Event::Empty(e)) => {
                let offset = reader.buffer_position() as usize;
                let (ns, _local) = reader.resolve(e.name(), false);
                parser.handle_start(&e, &ns, offset)?;
            }
            Ok(Event::End(e)) => {
                let tag = local_name(e.name());
                parser.handle_end(&tag)?;
            }
            Ok(Event::Text(e)) => {
                let text = e.unescape().unwrap_or_default();
                parser.handle_text(&text);
            }
            Ok(Event::Eof) => break,
            Err(e) => {
                return Err(CityGmlError::xml_error(
                    reader.buffer_position() as usize,
                    format!("{e}"),
                ));
            }
            _ => {}
        }
        buf.clear();
    }
    Ok(parser.finish())
}

/// Streaming XML parser state.
#[derive(Default)]
struct XmlParser {
    model: CityModel,
    /// Current linear ring being built.
    current_ring: Option<LinearRing>,
    /// Current surface being built.
    current_surface: Option<Surface>,
    /// Current solid being built.
    current_solid: Option<Solid>,
    /// Current geometry being built.
    current_geometry: Option<Geometry>,
    /// Buffer for text content.
    text_buf: String,
    /// Whether inside a `gml:posList` or `gml:pos`.
    in_pos_list: bool,
    /// Whether inside a thematic attribute value.
    in_thematic: bool,
    /// The current thematic key being parsed.
    thematic_key: Option<ThematicKey>,
    /// Object stack for nesting.
    object_stack: Vec<CityObject>,
    /// Semantic class of the enclosing `WallSurface`/`RoofSurface`/… element,
    /// applied to every polygon it contains.
    surface_class: SurfaceClass,
}

impl XmlParser {
    fn handle_start(
        &mut self,
        e: &quick_xml::events::BytesStart<'_>,
        ns: &ResolveResult<'_>,
        offset: usize,
    ) -> Result<(), CityGmlError> {
        let tag = local_name(e.name());
        let tag = tag.as_str();

        // Version detection from the root namespace.
        if tag == "CityModel" {
            if let ResolveResult::Bound(ns_bound) = ns {
                if ns_bound.0 == NS_CITYGML_3 {
                    self.model.version = CityGmlVersion::V3_0;
                } else {
                    self.model.version = CityGmlVersion::V2_0;
                }
            } else {
                self.model.version = CityGmlVersion::V2_0;
            }
        }

        // CRS from gml:Envelope @srsName.
        if tag == "Envelope" {
            for attr in e.attributes() {
                let attr = attr.map_err(|err| {
                    CityGmlError::xml_error(offset, format!("attribute error: {err}"))
                })?;
                let key = local_name(attr.key);
                if key == "srsName" {
                    let value = attr.unescape_value().unwrap_or_default().to_string();
                    self.model.crs = Some(Crs { identifier: value });
                }
            }
        }

        // City object: push onto the object stack.
        if is_city_object(tag) {
            let kind = CityObjectKind::from_tag(tag);
            let id = find_id(e);
            let obj = CityObject { kind, id, ..Default::default() };
            self.object_stack.push(obj);
        }

        // Geometry member (lodXMultiSurface, lodXSolid, …).
        if let Some(lod) = lod_from_tag(tag) {
            self.current_geometry = Some(Geometry::new(lod));
        }

        // Semantic surface boundary.
        if tag == "boundedBy" && !self.object_stack.is_empty() {
            self.current_surface = Some(Surface::default());
        }

        // Semantic surface element (WallSurface, RoofSurface, …): classifies
        // the polygons inside it.
        let class = SurfaceClass::from_tag(tag);
        if class != SurfaceClass::Generic && !self.object_stack.is_empty() {
            self.surface_class = class;
            if let Some(surface) = self.current_surface.as_mut() {
                surface.class = class;
            }
        }

        // Polygon: create a surface to hold the ring.
        if tag == "Polygon" && self.current_surface.is_none() {
            self.current_surface = Some(Surface { class: self.surface_class, ..Surface::default() });
        }

        // Solid.
        if tag == "Solid" {
            self.current_solid = Some(Solid::default());
        }

        // Linear ring.
        if tag == "LinearRing" {
            self.current_ring = Some(LinearRing::default());
        }

        // Thematic attribute elements.
        if is_thematic_element(tag) && !self.object_stack.is_empty() {
            self.in_thematic = true;
            self.thematic_key = Some(ThematicKey::from_tag(tag));
            self.text_buf.clear();
        }

        // Position list.
        if matches!(tag, "posList" | "pos") {
            self.in_pos_list = true;
            self.text_buf.clear();
        }

        Ok(())
    }

    fn handle_text(&mut self, text: &str) {
        if self.in_pos_list || self.in_thematic {
            self.text_buf.push_str(text);
        }
    }

    fn handle_end(&mut self, tag: &str) -> Result<(), CityGmlError> {
        match tag {
            "posList" | "pos" => {
                self.in_pos_list = false;
                self.parse_pos_list(&self.text_buf.clone())?;
                self.text_buf.clear();
            }
            "boundedBy" => {
                if let Some(surface) = self.current_surface.take() {
                    if let Some(geom) = self.current_geometry.as_mut() {
                        geom.surfaces.push(surface);
                    }
                }
            }
            "exterior" => {
                if let Some(ring) = self.current_ring.take() {
                    if let Some(surface) = self.current_surface.as_mut() {
                        surface.exterior = Some(ring);
                    }
                }
            }
            "interior" => {
                if let Some(ring) = self.current_ring.take() {
                    if let Some(surface) = self.current_surface.as_mut() {
                        surface.interiors.push(ring);
                    }
                }
            }
            "Polygon" => {
                if let Some(surface) = self.current_surface.take() {
                    if let Some(geom) = self.current_geometry.as_mut() {
                        geom.surfaces.push(surface);
                    }
                }
            }
            "Solid" => {
                if let Some(solid) = self.current_solid.take() {
                    if let Some(geom) = self.current_geometry.as_mut() {
                        geom.solids.push(solid);
                    }
                }
            }
            t if lod_from_tag(t).is_some() => {
                if let Some(geom) = self.current_geometry.take() {
                    if let Some(obj) = self.object_stack.last_mut() {
                        obj.geometries.push(geom);
                    }
                }
            }
            _ if is_city_object(tag) => {
                if let Some(obj) = self.object_stack.pop() {
                    if self.object_stack.is_empty() {
                        self.model.objects.push(obj);
                    } else {
                        self.object_stack.last_mut().unwrap().children.push(obj);
                    }
                }
            }
            t if SurfaceClass::from_tag(t) != SurfaceClass::Generic => {
                self.surface_class = SurfaceClass::Generic;
            }
            t if is_thematic_element(t) => {
                self.in_thematic = false;
                if let Some(key) = self.thematic_key.take() {
                    let value = self.text_buf.trim().to_string();
                    if !value.is_empty() {
                        if let Some(obj) = self.object_stack.last_mut() {
                            obj.attributes.push(ThematicAttribute { key, value });
                        }
                    }
                }
                self.text_buf.clear();
            }
            _ => {}
        }
        Ok(())
    }

    /// Parse a `gml:posList` string and attach the points to the current ring.
    fn parse_pos_list(&mut self, text: &str) -> Result<(), CityGmlError> {
        let tokens: Vec<&str> = text.split_whitespace().collect();
        let dim = 3; // CityGML uses 3D coordinates.
        if tokens.len() % dim != 0 {
            return Err(CityGmlError::xml_error(
                0,
                format!("posList has {} tokens, not divisible by {}", tokens.len(), dim),
            ));
        }
        let mut points = Vec::new();
        for chunk in tokens.chunks(dim) {
            let x = parse_f64(chunk[0])?;
            let y = parse_f64(chunk[1])?;
            let z = parse_f64(chunk[2])?;
            points.push(Point3 { x, y, z });
        }

        if let Some(ring) = self.current_ring.as_mut() {
            ring.points = points;
        }
        Ok(())
    }

    fn finish(mut self) -> CityModel {
        // Flush any remaining objects on the stack.
        while let Some(obj) = self.object_stack.pop() {
            if self.object_stack.is_empty() {
                self.model.objects.push(obj);
            } else {
                self.object_stack.last_mut().unwrap().children.push(obj);
            }
        }
        self.model
    }
}

/// Parse a single `f64` from a string token.
fn parse_f64(s: &str) -> Result<f64, CityGmlError> {
    s.parse::<f64>().map_err(|_| CityGmlError::invalid_number(s, 0))
}

/// Extract the local name from a qualified name (strip namespace prefix).
fn local_name(name: quick_xml::name::QName<'_>) -> String {
    let raw = name.0;
    match core::str::from_utf8(raw) {
        Ok(s) => match s.find(':') {
            Some(idx) => s[idx + 1..].to_string(),
            None => s.to_string(),
        },
        Err(_) => String::new(),
    }
}

/// Get the `id` attribute from a `BytesStart` event.
fn find_id(e: &quick_xml::events::BytesStart<'_>) -> Option<String> {
    for attr in e.attributes().flatten() {
        let key = local_name(attr.key);
        if key == "id" {
            return Some(attr.unescape_value().unwrap_or_default().to_string());
        }
    }
    None
}

/// Check if a tag is a city object element.
fn is_city_object(tag: &str) -> bool {
    matches!(
        tag.trim().to_ascii_lowercase().as_str(),
        "building"
            | "plantcover"
            | "solitaryvegetationobject"
            | "waterbody"
            | "road"
            | "railway"
            | "track"
            | "square"
            | "tunnel"
            | "bridge"
            | "landuse"
            | "cityfurniture"
    )
}

/// Check if a tag is a thematic attribute element.
fn is_thematic_element(tag: &str) -> bool {
    matches!(
        tag.trim().to_ascii_lowercase().as_str(),
        "function"
            | "yearofconstruction"
            | "energyperformance"
            | "rooftype"
            | "measuredheight"
            | "storeysaboveground"
            | "storeysbelowground"
    )
}

/// Extract the LOD level from a geometry element tag.
fn lod_from_tag(tag: &str) -> Option<Lod> {
    let lower = tag.trim().to_ascii_lowercase();
    for prefix in ["lod0", "lod1", "lod2", "lod3", "lod4"] {
        if lower.starts_with(prefix) {
            let lod_num = prefix[3..].parse::<u32>().ok()?;
            return Lod::from_u32(lod_num);
        }
    }
    None
}

/// Reader for CityGML documents, implementing [`tpt_io_core::Reader`].
#[derive(Debug, Clone, Default)]
pub struct CityGmlReader {
    validation: ValidationSettings,
}

impl CityGmlReader {
    /// Create a reader with default validation (syntactic + semantic).
    pub fn new() -> Self {
        Self { validation: ValidationSettings::new() }
    }

    /// Set the validation configuration.
    pub fn with_validation(mut self, validation: ValidationSettings) -> Self {
        self.validation = validation;
        self
    }
}

impl Reader<CityModel> for CityGmlReader {
    type Error = CityGmlError;

    fn parse(&self, input: &[u8]) -> Result<CityModel, CityGmlError> {
        let text = Encoding::detect(input)
            .as_str(input)
            .ok_or(CityGmlError::Unsupported("non-UTF-8 CityGML input"))?;
        let model = parse_str(text)?;
        self.validation.run(&CityGmlValidator, &model)?;
        Ok(model)
    }
}

/// Syntactic / semantic validation for a parsed [`CityModel`].
#[derive(Debug, Clone, Copy, Default)]
pub struct CityGmlValidator;

impl Validator<CityModel> for CityGmlValidator {
    type Error = CityGmlError;

    fn validate_syntactic(&self, value: &CityModel) -> Result<(), CityGmlError> {
        for obj in &value.objects {
            self.validate_object_syntactic(obj)?;
        }
        Ok(())
    }

    fn validate_semantic(&self, value: &CityModel) -> Result<(), CityGmlError> {
        for obj in &value.objects {
            self.validate_object_semantic(obj)?;
        }
        Ok(())
    }
}

impl CityGmlValidator {
    fn validate_object_syntactic(&self, obj: &CityObject) -> Result<(), CityGmlError> {
        for geom in &obj.geometries {
            if geom.surfaces.is_empty() && geom.solids.is_empty() {
                return Err(CityGmlError::missing_element("geometry content", 0));
            }
            for surface in &geom.surfaces {
                if surface.exterior.is_none() {
                    return Err(CityGmlError::missing_element("gml:exterior", 0));
                }
            }
        }
        for child in &obj.children {
            self.validate_object_syntactic(child)?;
        }
        Ok(())
    }

    fn validate_object_semantic(&self, obj: &CityObject) -> Result<(), CityGmlError> {
        for attr in &obj.attributes {
            if let ThematicKey::EnergyPerformance = attr.key {
                if attr.value.is_empty() {
                    return Err(CityGmlError::custom(
                        "energy performance attribute must not be empty",
                    ));
                }
            }
        }
        for child in &obj.children {
            self.validate_object_semantic(child)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tpt_io_core::Reader;

    /// A minimal CityGML 2.0 document with a single building.
    const CITYGML_20_MINIMAL: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<core:CityModel xmlns:core="http://www.opengis.net/citygml/2.0"
                xmlns:gml="http://www.opengis.net/gml">
  <core:cityObjectMember>
    <bldg:Building xmlns:bldg="http://www.opengis.net/citygml/building/2.0"
                   gml:id="building-1">
      <bldg:function>1000</bldg:function>
      <bldg:yearOfConstruction>2020</bldg:yearOfConstruction>
      <bldg:lod1Solid>
        <gml:Solid>
          <gml:exterior>
            <gml:CompositeSurface>
              <gml:surfaceMember>
                <gml:Polygon>
                  <gml:exterior>
                    <gml:LinearRing>
                      <gml:posList>0 0 0 1 0 0 1 1 0 0 1 0 0 0 0</gml:posList>
                    </gml:LinearRing>
                  </gml:exterior>
                </gml:Polygon>
              </gml:surfaceMember>
            </gml:CompositeSurface>
          </gml:exterior>
        </gml:Solid>
      </bldg:lod1Solid>
    </bldg:Building>
  </core:cityObjectMember>
</core:CityModel>"#;

    #[test]
    fn parses_citygml_20_minimal() {
        let model = parse_str(CITYGML_20_MINIMAL).unwrap();
        assert_eq!(model.version, CityGmlVersion::V2_0);
        assert_eq!(model.objects.len(), 1);
        let obj = &model.objects[0];
        assert_eq!(obj.kind, CityObjectKind::Building);
        assert_eq!(obj.id.as_deref(), Some("building-1"));
        assert_eq!(obj.geometries.len(), 1);
        assert_eq!(obj.geometries[0].lod, Lod::Lod1);
        assert_eq!(obj.attributes.len(), 2);
        assert_eq!(obj.attributes[0].value, "1000");
        assert_eq!(obj.attributes[1].value, "2020");
    }

    #[test]
    fn parses_citygml_30_root_namespace() {
        let src = r#"<?xml version="1.0" encoding="UTF-8"?>
<core:CityModel xmlns:core="http://www.opengis.net/citygml/3.0"
                xmlns:gml="http://www.opengis.net/gml">
  <core:cityObjectMember>
    <bldg:Building xmlns:bldg="http://www.opengis.net/citygml/building/3.0"
                   gml:id="b-2">
      <bldg:lod2MultiSurface>
        <gml:MultiSurface>
          <gml:surfaceMember>
            <gml:Polygon>
              <gml:exterior>
                <gml:LinearRing>
                  <gml:posList>0 0 10 5 0 10 5 5 10 0 5 10 0 0 10</gml:posList>
                </gml:LinearRing>
              </gml:exterior>
            </gml:Polygon>
          </gml:surfaceMember>
        </gml:MultiSurface>
      </bldg:lod2MultiSurface>
    </bldg:Building>
  </core:cityObjectMember>
</core:CityModel>"#;
        let model = parse_str(src).unwrap();
        assert_eq!(model.version, CityGmlVersion::V3_0);
        assert_eq!(model.objects.len(), 1);
        assert_eq!(model.objects[0].geometries[0].lod, Lod::Lod2);
    }

    #[test]
    fn detects_crs_from_envelope() {
        let src = r#"<?xml version="1.0" encoding="UTF-8"?>
<core:CityModel xmlns:core="http://www.opengis.net/citygml/2.0"
                xmlns:gml="http://www.opengis.net/gml">
  <core:boundedBy>
    <gml:Envelope srsName="urn:ogc:def:crs:EPSG::25832" srsDimension="3">
      <gml:lowerCorner>0 0 0</gml:lowerCorner>
      <gml:upperCorner>100 100 50</gml:upperCorner>
    </gml:Envelope>
  </core:boundedBy>
</core:CityModel>"#;
        let model = parse_str(src).unwrap();
        let crs = model.crs.as_ref().expect("CRS should be detected");
        assert_eq!(crs.identifier, "urn:ogc:def:crs:EPSG::25832");
        assert_eq!(crs.epsg_code(), Some(25832));
    }

    #[test]
    fn reader_runs_validation() {
        let reader = CityGmlReader::new();
        let model = reader.parse(CITYGML_20_MINIMAL.as_bytes()).unwrap();
        assert_eq!(model.objects.len(), 1);
    }

    #[test]
    fn syntactic_validation_rejects_surface_without_exterior() {
        let model = CityModel {
            version: CityGmlVersion::V2_0,
            crs: None,
            objects: vec![CityObject {
                kind: CityObjectKind::Building,
                id: None,
                geometries: vec![Geometry::new(Lod::Lod2)],
                attributes: Vec::new(),
                children: Vec::new(),
            }],
        };
        let result = CityGmlValidator.validate_syntactic(&model);
        assert!(result.is_err());
    }

    #[test]
    fn parses_multiple_city_objects() {
        let src = r#"<?xml version="1.0" encoding="UTF-8"?>
<core:CityModel xmlns:core="http://www.opengis.net/citygml/2.0"
                xmlns:gml="http://www.opengis.net/gml">
  <core:cityObjectMember>
    <bldg:Building xmlns:bldg="http://www.opengis.net/citygml/building/2.0"
                   gml:id="b-1">
      <bldg:lod1Solid>
        <gml:Solid>
          <gml:exterior>
            <gml:CompositeSurface>
              <gml:surfaceMember>
                <gml:Polygon>
                  <gml:exterior>
                    <gml:LinearRing>
                      <gml:posList>0 0 0 1 0 0 1 1 0 0 1 0 0 0 0</gml:posList>
                    </gml:LinearRing>
                  </gml:exterior>
                </gml:Polygon>
              </gml:surfaceMember>
            </gml:CompositeSurface>
          </gml:exterior>
        </gml:Solid>
      </bldg:lod1Solid>
    </bldg:Building>
  </core:cityObjectMember>
  <core:cityObjectMember>
    <tran:Road xmlns:tran="http://www.opengis.net/citygml/transportation/2.0"
               gml:id="r-1">
      <tran:lod1MultiSurface>
        <gml:MultiSurface>
          <gml:surfaceMember>
            <gml:Polygon>
              <gml:exterior>
                <gml:LinearRing>
                  <gml:posList>0 0 0 10 0 0 10 2 0 0 2 0 0 0 0</gml:posList>
                </gml:LinearRing>
              </gml:exterior>
            </gml:Polygon>
          </gml:surfaceMember>
        </gml:MultiSurface>
      </tran:lod1MultiSurface>
    </tran:Road>
  </core:cityObjectMember>
</core:CityModel>"#;
        let model = parse_str(src).unwrap();
        assert_eq!(model.objects.len(), 2);
        assert_eq!(model.objects[0].kind, CityObjectKind::Building);
        assert_eq!(model.objects[1].kind, CityObjectKind::Transportation);
    }

    #[test]
    fn empty_citymodel() {
        let src = r#"<?xml version="1.0" encoding="UTF-8"?>
<core:CityModel xmlns:core="http://www.opengis.net/citygml/2.0"
                xmlns:gml="http://www.opengis.net/gml">
  <core:boundedBy>
    <gml:Envelope/>
  </core:boundedBy>
</core:CityModel>"#;
        let model = parse_str(src).unwrap();
        assert_eq!(model.objects.len(), 0);
        assert_eq!(model.version, CityGmlVersion::V2_0);
    }

    #[test]
    fn parses_poslist_points() {
        let src = r#"<?xml version="1.0" encoding="UTF-8"?>
<core:CityModel xmlns:core="http://www.opengis.net/citygml/2.0"
                xmlns:gml="http://www.opengis.net/gml">
  <core:cityObjectMember>
    <bldg:Building xmlns:bldg="http://www.opengis.net/citygml/building/2.0"
                   gml:id="b-1">
      <bldg:lod2MultiSurface>
        <gml:MultiSurface>
          <gml:surfaceMember>
            <gml:Polygon>
              <gml:exterior>
                <gml:LinearRing>
                  <gml:posList>0 0 0 10 0 0 10 10 0 0 10 0 0 0 0</gml:posList>
                </gml:LinearRing>
              </gml:exterior>
            </gml:Polygon>
          </gml:surfaceMember>
        </gml:MultiSurface>
      </bldg:lod2MultiSurface>
    </bldg:Building>
  </core:cityObjectMember>
</core:CityModel>"#;
        let model = parse_str(src).unwrap();
        let geom = &model.objects[0].geometries[0];
        assert!(!geom.surfaces.is_empty());
        let surface = &geom.surfaces[0];
        let ring = surface.exterior.as_ref().expect("exterior ring should exist");
        assert_eq!(ring.points.len(), 5);
        assert_eq!(ring.points[0], Point3 { x: 0.0, y: 0.0, z: 0.0 });
        assert_eq!(ring.points[1], Point3 { x: 10.0, y: 0.0, z: 0.0 });
    }
}
