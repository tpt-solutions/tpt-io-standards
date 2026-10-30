//! IFC schema version detection and migration utilities.
//!
//! Every IFC file declares its schema in the `FILE_SCHEMA` header entry
//! (e.g. `('IFC4')`, `('IFC4X3')`, `('IFC2X3')`). This module parses that
//! declaration into a typed [`IfcSchemaVersion`], detects it from a parsed
//! file, and provides migration helpers that map entity/property names
//! between the most common schema versions (IFC2X3 ↔ IFC4 ↔ IFC4X3).

use crate::model::IfcFile;

/// The IFC schema version declared by a file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IfcSchemaVersion {
    /// IFC2X3 (the legacy 2.3 schema).
    Ifc2X3,
    /// IFC4 (ISO 16739-1:2018).
    Ifc4,
    /// IFC4X3 (ISO 16739-1:2023, the 4.3 addendum).
    Ifc4X3,
    /// A schema version the parser does not specifically recognise.
    Unknown,
}

impl IfcSchemaVersion {
    /// Parse a schema identifier string (e.g. `"IFC4"`) into a version.
    pub fn parse(s: &str) -> Self {
        match s.trim().to_uppercase().as_str() {
            "IFC2X3" => IfcSchemaVersion::Ifc2X3,
            "IFC4" | "IFC4X0" => IfcSchemaVersion::Ifc4,
            "IFC4X3" | "IFC4X3_ADD1" | "IFC4X3_ADD2" | "IFC4X3_TC1" => IfcSchemaVersion::Ifc4X3,
            _ => IfcSchemaVersion::Unknown,
        }
    }

    /// Return the canonical schema identifier string for this version.
    pub fn as_str(self) -> &'static str {
        match self {
            IfcSchemaVersion::Ifc2X3 => "IFC2X3",
            IfcSchemaVersion::Ifc4 => "IFC4",
            IfcSchemaVersion::Ifc4X3 => "IFC4X3",
            IfcSchemaVersion::Unknown => "UNKNOWN",
        }
    }

    /// Return `true` if this is a known (non-unknown) version.
    pub fn is_known(self) -> bool {
        !matches!(self, IfcSchemaVersion::Unknown)
    }
}

impl core::fmt::Display for IfcSchemaVersion {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Detect the schema version of a parsed IFC file.
///
/// Returns the first recognised schema in the `FILE_SCHEMA` list, or
/// [`IfcSchemaVersion::Unknown`] if the list is empty or contains no known
/// version.
pub fn detect_schema(file: &IfcFile) -> IfcSchemaVersion {
    for s in &file.header.schemas {
        let v = IfcSchemaVersion::parse(s);
        if v.is_known() {
            return v;
        }
    }
    IfcSchemaVersion::Unknown
}

/// A migration that rewrites entity type names between schema versions.
///
/// IFC4X3 introduced a number of renamed entities (e.g. `IfcBuildingElement` →
/// `IfcBuiltElement`). This mapper applies those renames so that downstream
/// code written against one schema can read files written against another.
///
/// The mapping is intentionally conservative: it covers the most impactful
/// renames and leaves everything else untouched.
pub struct SchemaMigration {
    /// The source schema version.
    pub source: IfcSchemaVersion,
    /// The target schema version.
    pub target: IfcSchemaVersion,
}

impl SchemaMigration {
    /// Create a migration from `source` to `target`.
    pub fn new(source: IfcSchemaVersion, target: IfcSchemaVersion) -> Self {
        Self { source, target }
    }

    /// Create a migration that converts `file`'s declared schema to `target`.
    pub fn from_file(file: &IfcFile, target: IfcSchemaVersion) -> Self {
        Self { source: detect_schema(file), target }
    }

    /// Map a single entity type name from the source schema to the target.
    ///
    /// If no rename applies, the input is returned unchanged.
    pub fn map_entity<'a>(&self, type_name: &'a str) -> &'a str {
        match (self.source, self.target) {
            (IfcSchemaVersion::Ifc4, IfcSchemaVersion::Ifc4X3) => map_ifc4_to_ifc4x3(type_name),
            (IfcSchemaVersion::Ifc4X3, IfcSchemaVersion::Ifc4) => map_ifc4x3_to_ifc4(type_name),
            (IfcSchemaVersion::Ifc2X3, IfcSchemaVersion::Ifc4) => map_ifc2x3_to_ifc4(type_name),
            (IfcSchemaVersion::Ifc4, IfcSchemaVersion::Ifc2X3) => map_ifc4_to_ifc2x3(type_name),
            // No mapping needed (same version or unknown).
            _ => type_name,
        }
    }

    /// Map a property set name from the source schema to the target.
    pub fn map_property_set<'b>(&self, name: &'b str) -> &'b str {
        // Property set names are generally stable across versions; apply only
        // the well-known renames. Since `replace` would allocate, we match on
        // the known renames and return a static replacement when applicable.
        match (self.source, self.target) {
            (IfcSchemaVersion::Ifc4, IfcSchemaVersion::Ifc4X3)
                if name == "Pset_BuildingElement" =>
            {
                "Pset_BuiltElement"
            }
            _ => name,
        }
    }
}

/// IFC4 → IFC4X3 entity renames (a representative subset).
fn map_ifc4_to_ifc4x3(name: &str) -> &str {
    match name {
        "IFCBUILDINGELEMENT" => "IFCBUILTELEMENT",
        "IFCBUILDINGELEMENTPART" => "IFCBUILTELEMENTPART",
        "IFCBUILDINGELEMENTPROXY" => "IFCBUILTELEMENTPROXY",
        "IFCBUILDINGELEMENTPROXYTYPE" => "IFCBUILTELEMENTPROXYTYPE",
        "IFCBUILDINGELEMENTTYPE" => "IFCBUILTELEMENTTYPE",
        "IFCREINFORCINGBAR" => "IFCREINFORCINGBAR",
        other => other,
    }
}

/// IFC4X3 → IFC4 entity renames (inverse of [`map_ifc4_to_ifc4x3`]).
fn map_ifc4x3_to_ifc4(name: &str) -> &str {
    match name {
        "IFCBUILTELEMENT" => "IFCBUILDINGELEMENT",
        "IFCBUILTELEMENTPART" => "IFCBUILDINGELEMENTPART",
        "IFCBUILTELEMENTPROXY" => "IFCBUILDINGELEMENTPROXY",
        "IFCBUILTELEMENTPROXYTYPE" => "IFCBUILDINGELEMENTPROXYTYPE",
        "IFCBUILTELEMENTTYPE" => "IFCBUILDINGELEMENTTYPE",
        other => other,
    }
}

/// IFC2X3 → IFC4 entity renames (a representative subset).
fn map_ifc2x3_to_ifc4(name: &str) -> &str {
    match name {
        "IFCBUILDINGELEMENTCOMPONENT" => "IFCBUILDINGELEMENTPART",
        "IFCBUILDINGELEMENTPROXY" => "IFCBUILDINGELEMENTPROXY",
        other => other,
    }
}

/// IFC4 → IFC2X3 entity renames (inverse of [`map_ifc2x3_to_ifc4`]).
fn map_ifc4_to_ifc2x3(name: &str) -> &str {
    match name {
        "IFCBUILDINGELEMENTPART" => "IFCBUILDINGELEMENTCOMPONENT",
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(not(feature = "std"))]
    use alloc::string::ToString;
    use crate::model::IfcHeader;

    fn file_with_schema(schema: &str) -> IfcFile {
        IfcFile {
            header: IfcHeader { schemas: vec![schema.to_string()], ..IfcHeader::default() },
            ..IfcFile::default()
        }
    }

    #[test]
    fn parse_known_versions() {
        assert_eq!(IfcSchemaVersion::parse("IFC4"), IfcSchemaVersion::Ifc4);
        assert_eq!(IfcSchemaVersion::parse("ifc4"), IfcSchemaVersion::Ifc4);
        assert_eq!(IfcSchemaVersion::parse("IFC4X3"), IfcSchemaVersion::Ifc4X3);
        assert_eq!(IfcSchemaVersion::parse("IFC2X3"), IfcSchemaVersion::Ifc2X3);
        assert_eq!(IfcSchemaVersion::parse("IFC5"), IfcSchemaVersion::Unknown);
    }

    #[test]
    fn detect_from_file() {
        assert_eq!(detect_schema(&file_with_schema("IFC4")), IfcSchemaVersion::Ifc4);
        assert_eq!(detect_schema(&file_with_schema("IFC4X3")), IfcSchemaVersion::Ifc4X3);
        assert_eq!(detect_schema(&file_with_schema("IFC5")), IfcSchemaVersion::Unknown);
        assert_eq!(detect_schema(&IfcFile::default()), IfcSchemaVersion::Unknown);
    }

    #[test]
    fn migrate_ifc4_to_ifc4x3() {
        let m = SchemaMigration::new(IfcSchemaVersion::Ifc4, IfcSchemaVersion::Ifc4X3);
        assert_eq!(m.map_entity("IFCBUILDINGELEMENT"), "IFCBUILTELEMENT");
        assert_eq!(m.map_entity("IFCWALL"), "IFCWALL");
    }

    #[test]
    fn migrate_ifc4x3_to_ifc4() {
        let m = SchemaMigration::new(IfcSchemaVersion::Ifc4X3, IfcSchemaVersion::Ifc4);
        assert_eq!(m.map_entity("IFCBUILTELEMENT"), "IFCBUILDINGELEMENT");
        assert_eq!(m.map_entity("IFCWALL"), "IFCWALL");
    }

    #[test]
    fn migrate_from_file() {
        let file = file_with_schema("IFC4");
        let m = SchemaMigration::from_file(&file, IfcSchemaVersion::Ifc4X3);
        assert_eq!(m.source, IfcSchemaVersion::Ifc4);
        assert_eq!(m.target, IfcSchemaVersion::Ifc4X3);
        assert_eq!(m.map_entity("IFCBUILDINGELEMENT"), "IFCBUILTELEMENT");
    }

    #[test]
    fn no_op_migration_for_same_version() {
        let m = SchemaMigration::new(IfcSchemaVersion::Ifc4, IfcSchemaVersion::Ifc4);
        assert_eq!(m.map_entity("IFCBUILDINGELEMENT"), "IFCBUILDINGELEMENT");
    }

    #[test]
    fn display_and_str() {
        assert_eq!(IfcSchemaVersion::Ifc4.as_str(), "IFC4");
        assert_eq!(format!("{}", IfcSchemaVersion::Ifc4X3), "IFC4X3");
    }
}
