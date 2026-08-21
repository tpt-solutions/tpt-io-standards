//! IFC data model (ISO 10303-21 STEP-Physical File entity instances).
//!
//! The model captures the generic envelope: every `#id = ENTITY(...)` instance
//! is stored with its numeric id, entity name, and attribute list. Attribute
//! values are kept in a tree-shaped [`Value`] so nested lists and inline
//! entities (common in IFC geometry) round-trip losslessly.

use alloc::string::String;
use alloc::vec::Vec;

/// A single attribute value within an entity instance.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    /// `$` — unspecified / not provided.
    Unset,
    /// An integer literal.
    Int(i64),
    /// A real (floating-point) literal.
    Real(f64),
    /// A string literal (single-quoted, `''` escaped).
    Str(String),
    /// An instance reference `#id`.
    Ref(u64),
    /// An enumeration value `.NAME.`.
    Enum(String),
    /// An ordered list of values `(a, b, c)`.
    List(Vec<Value>),
    /// An inline (anonymous) entity `NAME(...)`.
    Entity(String, Vec<Value>),
}

/// A parsed `#id = ENTITY(...)` instance.
#[derive(Debug, Clone, PartialEq)]
pub struct EntityInstance {
    /// The instance id (the number after `#`).
    pub id: u64,
    /// The entity type name (e.g. `IFCWALL`, `IFCPROJECT`).
    pub entity: String,
    /// The attribute values, in declaration order.
    pub attributes: Vec<Value>,
}

/// A fully parsed STEP-Physical File.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct IfcDocument {
    /// All `#id`-keyed entity instances (HEADER and DATA sections).
    pub instances: Vec<EntityInstance>,
}

impl IfcDocument {
    /// Resolve an instance by its `#id`.
    pub fn by_id(&self, id: u64) -> Option<&EntityInstance> {
        self.instances.iter().find(|e| e.id == id)
    }
    /// Iterate every instance of a given entity type (case-insensitive).
    pub fn by_entity<'a>(&'a self, name: &'a str) -> impl Iterator<Item = &'a EntityInstance> {
        self.instances
            .iter()
            .filter(move |e| e.entity.eq_ignore_ascii_case(name))
    }
}
