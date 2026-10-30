//! Strongly-typed data model for a parsed CityGML 2.0 / 3.0 document.
//!
//! The model mirrors the CityGML conceptual schema: a [`CityModel`] contains
//! [`CityObject`]s (buildings, vegetation, water, transportation, …), each of
//! which may carry geometry at one or more LODs (0–4), semantic surface
//! classifications, and thematic attributes.

use alloc::string::String;
use alloc::vec::Vec;

/// CityGML schema version detected during parsing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CityGmlVersion {
    /// CityGML 2.0 (OGC 08-007r2).
    #[default]
    V2_0,
    /// CityGML 3.0 (OGC 20-010).
    V3_0,
}

/// Level of Detail (LOD) for a geometric representation.
///
/// CityGML defines LOD 0 (regional) through LOD 4 (interior). Values outside
/// this range are rejected by the parser.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Lod {
    /// LOD 0 — 2.5D regional footprint.
    #[default]
    Lod0,
    /// LOD 1 — block model.
    Lod1,
    /// LOD 2 — differentiated roof structures.
    Lod2,
    /// LOD 3 — architectural details (windows, doors).
    Lod3,
    /// LOD 4 — interior structures.
    Lod4,
}

impl Lod {
    /// Convert a numeric LOD value (0–4) into a [`Lod`].
    pub fn from_u32(value: u32) -> Option<Self> {
        match value {
            0 => Some(Lod::Lod0),
            1 => Some(Lod::Lod1),
            2 => Some(Lod::Lod2),
            3 => Some(Lod::Lod3),
            4 => Some(Lod::Lod4),
            _ => None,
        }
    }

    /// The numeric LOD value (0–4).
    pub fn as_u32(self) -> u32 {
        match self {
            Lod::Lod0 => 0,
            Lod::Lod1 => 1,
            Lod::Lod2 => 2,
            Lod::Lod3 => 3,
            Lod::Lod4 => 4,
        }
    }
}

/// Semantic surface classification for a bounded surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SurfaceClass {
    /// Wall surface.
    Wall,
    /// Roof surface.
    Roof,
    /// Ground surface.
    Ground,
    /// Window (opening).
    Window,
    /// Door (opening).
    Door,
    /// Closure / outer ceiling surface.
    Closure,
    /// Generic / unclassified surface.
    #[default]
    Generic,
}

impl SurfaceClass {
    /// Parse a surface class from a CityGML `SurfaceClassificator` string.
    pub fn from_tag(tag: &str) -> Self {
        match tag.trim().to_ascii_lowercase().as_str() {
            "wall" | "wallsurface" => SurfaceClass::Wall,
            "roof" | "roofsurface" => SurfaceClass::Roof,
            "ground" | "groundsurface" => SurfaceClass::Ground,
            "window" => SurfaceClass::Window,
            "door" => SurfaceClass::Door,
            "closure" | "closuresurface" => SurfaceClass::Closure,
            _ => SurfaceClass::Generic,
        }
    }
}

/// Thematic attribute key for a city object.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum ThematicKey {
    /// Building / land use function code.
    #[default]
    Function,
    /// Year of construction.
    YearBuilt,
    /// Energy performance class or rating.
    EnergyPerformance,
    /// Roof type identifier.
    RoofType,
    /// Measured height (in CRS units).
    Height,
    /// Number of storeys above ground.
    StoreysAboveGround,
    /// Free-form attribute key.
    Custom(String),
}

impl ThematicKey {
    /// Parse a thematic key from a CityGML element name or attribute name.
    pub fn from_tag(tag: &str) -> Self {
        match tag.trim().to_ascii_lowercase().as_str() {
            "function" => ThematicKey::Function,
            "yearofconstruction" | "year_built" => ThematicKey::YearBuilt,
            "energyperformance" | "energy_performance" => ThematicKey::EnergyPerformance,
            "rooftype" | "roof_type" => ThematicKey::RoofType,
            "measuredheight" | "height" => ThematicKey::Height,
            "storeysaboveground" | "storeys_above_ground" => ThematicKey::StoreysAboveGround,
            other => ThematicKey::Custom(other.to_string()),
        }
    }
}

/// A single thematic attribute (key + value).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ThematicAttribute {
    /// The attribute key.
    pub key: ThematicKey,
    /// The attribute value (as a string; numeric values are parsed on demand).
    pub value: String,
}

/// A 3D point in the active CRS.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Point3 {
    /// X coordinate (easting or longitude).
    pub x: f64,
    /// Y coordinate (northing or latitude).
    pub y: f64,
    /// Z coordinate (elevation).
    pub z: f64,
}

/// A linear ring (closed polygon boundary).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct LinearRing {
    /// Ordered vertices; the ring is implicitly closed.
    pub points: Vec<Point3>,
}

/// A planar surface bounded by one or more [`LinearRing`]s.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Surface {
    /// Exterior ring.
    pub exterior: Option<LinearRing>,
    /// Interior rings (holes).
    pub interiors: Vec<LinearRing>,
    /// Semantic classification of this surface.
    pub class: SurfaceClass,
}

/// A solid geometry (closed shell of surfaces).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Solid {
    /// Bounding surfaces of the solid.
    pub surfaces: Vec<Surface>,
}

/// Geometric representation at a specific LOD.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Geometry {
    /// The LOD level.
    pub lod: Lod,
    /// Surfaces (for LOD 0–2 surface-based representations).
    pub surfaces: Vec<Surface>,
    /// Solids (for LOD 1–4 volumetric representations).
    pub solids: Vec<Solid>,
}

impl Geometry {
    /// Create a new geometry at the given LOD.
    pub fn new(lod: Lod) -> Self {
        Self { lod, ..Default::default() }
    }
}

/// The kind of a city object.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CityObjectKind {
    /// Building (the most common city object).
    #[default]
    Building,
    /// Vegetation (plant cover, solitary tree).
    Vegetation,
    /// Water body.
    Water,
    /// Transportation (road, railway, square).
    Transportation,
    /// Tunnel.
    Tunnel,
    /// Bridge.
    Bridge,
    /// Land use / terrain.
    LandUse,
    /// City furniture (lamp post, bench, …).
    CityFurniture,
    /// Generic / unclassified city object.
    Generic,
}

impl CityObjectKind {
    /// Parse a city object kind from a CityGML element name.
    pub fn from_tag(tag: &str) -> Self {
        match tag.trim().to_ascii_lowercase().as_str() {
            "building" => CityObjectKind::Building,
            "plantcover" | "solitaryvegetationobject" | "vegetation" => CityObjectKind::Vegetation,
            "waterbody" | "water" => CityObjectKind::Water,
            "road" | "railway" | "track" | "square" | "transportation" => {
                CityObjectKind::Transportation
            }
            "tunnel" => CityObjectKind::Tunnel,
            "bridge" => CityObjectKind::Bridge,
            "landuse" => CityObjectKind::LandUse,
            "cityfurniture" => CityObjectKind::CityFurniture,
            _ => CityObjectKind::Generic,
        }
    }
}

/// A single city object (building, tree, road, …).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct CityObject {
    /// The object kind.
    pub kind: CityObjectKind,
    /// Optional gml:id identifier.
    pub id: Option<String>,
    /// Geometry representations at various LODs.
    pub geometries: Vec<Geometry>,
    /// Thematic attributes.
    pub attributes: Vec<ThematicAttribute>,
    /// Nested child objects (e.g. building parts, building installations).
    pub children: Vec<CityObject>,
}

/// Coordinate reference system identifier (e.g. `"urn:ogc:def:crs:EPSG::25832"`).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Crs {
    /// The raw CRS identifier string.
    pub identifier: String,
}

impl Crs {
    /// Extract the EPSG code from a CRS identifier, if present.
    pub fn epsg_code(&self) -> Option<u32> {
        self.identifier.rsplit("::").next().and_then(|s| s.parse::<u32>().ok())
    }
}

/// A fully parsed CityGML document.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct CityModel {
    /// Detected CityGML schema version.
    pub version: CityGmlVersion,
    /// Active coordinate reference system.
    pub crs: Option<Crs>,
    /// Top-level city objects.
    pub objects: Vec<CityObject>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lod_round_trip() {
        for v in 0..=4 {
            let lod = Lod::from_u32(v).unwrap();
            assert_eq!(lod.as_u32(), v);
        }
        assert!(Lod::from_u32(5).is_none());
    }

    #[test]
    fn surface_class_from_tag() {
        assert_eq!(SurfaceClass::from_tag("WallSurface"), SurfaceClass::Wall);
        assert_eq!(SurfaceClass::from_tag("Roof"), SurfaceClass::Roof);
        assert_eq!(SurfaceClass::from_tag("Window"), SurfaceClass::Window);
        assert_eq!(SurfaceClass::from_tag("unknown"), SurfaceClass::Generic);
    }

    #[test]
    fn city_object_kind_from_tag() {
        assert_eq!(CityObjectKind::from_tag("Building"), CityObjectKind::Building);
        assert_eq!(CityObjectKind::from_tag("PlantCover"), CityObjectKind::Vegetation);
        assert_eq!(CityObjectKind::from_tag("Road"), CityObjectKind::Transportation);
    }

    #[test]
    fn epsg_code_extraction() {
        let crs = Crs { identifier: "urn:ogc:def:crs:EPSG::25832".to_string() };
        assert_eq!(crs.epsg_code(), Some(25832));
        let crs_no_epsg = Crs { identifier: "local".to_string() };
        assert_eq!(crs_no_epsg.epsg_code(), None);
    }
}
