//! Strongly-typed data model for a parsed IFC (STEP-Physical File) document.
//!
//! The model mirrors the SPF structure: a [`IfcFile`] owns an [`IfcHeader`]
//! (the HEADER section's `FILE_DESCRIPTION`, `FILE_NAME`, `FILE_SCHEMA`) and a
//! list of entity instances. Each [`EntityInstance`] carries its numeric id
//! (the `#n`), its type name (e.g. `IFCWALL`), and a list of typed
//! [`IfcValue`] arguments.

use alloc::string::String;
use alloc::vec::Vec;

/// A single typed argument value of an IFC entity instance.
///
/// SPF maps a rich EXPRESS-typed argument space onto a small set of literal
/// forms; this enum captures every form the parser can produce. `$` (derived)
/// and `*` (re-derived) both collapse to [`IfcValue::Derived`].
#[derive(Debug, Clone, PartialEq)]
pub enum IfcValue {
    /// Omitted / derived value (`$` or `*`).
    Derived,
    /// A reference to another entity instance (`#id`).
    Ref(usize),
    /// An enumeration literal (`.ENUM.`), stored without the surrounding dots.
    Enum(String),
    /// A string literal (`'…'`).
    String(String),
    /// An integer literal.
    Integer(i64),
    /// A real (floating-point) literal.
    Real(f64),
    /// A boolean — IFC encodes `.T.` / `.F.` as enumerations, surfaced here.
    Bool(bool),
    /// An aggregate / list of values (`(…)`).
    List(Vec<IfcValue>),
}

impl IfcValue {
    /// Returns `true` if this is the derived/omitted marker.
    pub fn is_derived(&self) -> bool {
        matches!(self, IfcValue::Derived)
    }

    /// If this is a reference, return its id.
    pub fn as_ref(&self) -> Option<usize> {
        match self {
            IfcValue::Ref(id) => Some(*id),
            _ => None,
        }
    }

    /// If this is a string, return its content.
    pub fn as_string(&self) -> Option<&str> {
        match self {
            IfcValue::String(s) => Some(s),
            _ => None,
        }
    }

    /// If this is an integer, return its value.
    pub fn as_integer(&self) -> Option<i64> {
        match self {
            IfcValue::Integer(i) => Some(*i),
            _ => None,
        }
    }

    /// If this is a real, return its value.
    pub fn as_real(&self) -> Option<f64> {
        match self {
            IfcValue::Real(r) => Some(*r),
            _ => None,
        }
    }

    /// If this is a boolean, return its value.
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            IfcValue::Bool(b) => Some(*b),
            _ => None,
        }
    }

    /// If this is a list, return its elements.
    pub fn as_list(&self) -> Option<&[IfcValue]> {
        match self {
            IfcValue::List(v) => Some(v),
            _ => None,
        }
    }
}

/// A single entity instance: `#id = TYPE(arg0, arg1, …);`.
#[derive(Debug, Clone, PartialEq)]
pub struct EntityInstance {
    /// The numeric instance id (`#n`).
    pub id: usize,
    /// The entity type name, e.g. `IFCWALL`.
    pub type_name: String,
    /// The positional arguments.
    pub args: Vec<IfcValue>,
}

impl EntityInstance {
    /// Create a new entity instance.
    pub fn new(id: usize, type_name: impl Into<String>, args: Vec<IfcValue>) -> Self {
        Self { id, type_name: type_name.into(), args }
    }

    /// Return the argument at `index`, if present.
    pub fn arg(&self, index: usize) -> Option<&IfcValue> {
        self.args.get(index)
    }

    /// Return the number of arguments.
    pub fn arg_count(&self) -> usize {
        self.args.len()
    }
}

/// The `FILE_DESCRIPTION` header entry.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct FileDescription {
    /// The view definition list, e.g. `["CoordinationView"]`.
    pub descriptions: Vec<String>,
    /// The implementation level, e.g. `"2;1"`.
    pub implementation_level: String,
}

/// The `FILE_NAME` header entry.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct FileName {
    /// The file name / path.
    pub name: String,
    /// The ISO 8601 timestamp.
    pub time_stamp: String,
    /// The author list.
    pub authors: Vec<String>,
    /// The organization list.
    pub organizations: Vec<String>,
    /// The preprocessor version.
    pub preprocessor_version: String,
    /// The originating system.
    pub originating_system: String,
    /// The authorization string.
    pub authorization: String,
}

/// The complete HEADER section.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct IfcHeader {
    /// The `FILE_DESCRIPTION` entry.
    pub description: FileDescription,
    /// The `FILE_NAME` entry.
    pub file_name: FileName,
    /// The `FILE_SCHEMA` entry — the list of schema identifiers, e.g.
    /// `["IFC4"]` or `["IFC4X3"]`.
    pub schemas: Vec<String>,
}

/// A fully parsed IFC document.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct IfcFile {
    /// The HEADER section.
    pub header: IfcHeader,
    /// The entity instances from the DATA section(s), in file order.
    pub instances: Vec<EntityInstance>,
}

impl IfcFile {
    /// Create an empty file with the given header.
    pub fn new(header: IfcHeader) -> Self {
        Self { header, instances: Vec::new() }
    }

    /// Return the number of entity instances.
    pub fn instance_count(&self) -> usize {
        self.instances.len()
    }

    /// Look up an entity instance by its numeric id.
    pub fn instance(&self, id: usize) -> Option<&EntityInstance> {
        self.instances.iter().find(|e| e.id == id)
    }

    /// Return all instances of a given type name (case-sensitive).
    pub fn instances_of<'a>(
        &'a self,
        type_name: &'a str,
    ) -> impl Iterator<Item = &'a EntityInstance> + 'a {
        self.instances.iter().filter(move |e| e.type_name == type_name)
    }

    /// Return the schema identifier from the header, if any.
    ///
    /// Most files declare a single schema (e.g. `"IFC4"`); this returns the
    /// first entry of the `FILE_SCHEMA` list.
    pub fn schema(&self) -> Option<&str> {
        self.header.schemas.first().map(|s| s.as_str())
    }
}
