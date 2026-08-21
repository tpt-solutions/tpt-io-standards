//! FHIR data model (JSON, R4/R5).
//!
//! FHIR defines ~150 resource types; rather than model each one, this crate
//! captures the common envelope (a resource carries a `resourceType` and `id`,
//! optional `meta`, and its full JSON body) and the `Bundle` container used to
//! batch resources. Resource-specific fields are available through the
//! preserved `raw` JSON.

use alloc::string::String;
use alloc::vec::Vec;

use tpt_io_core::json::Json;

/// A single FHIR resource (any of the ~150 types).
#[derive(Debug, Clone, PartialEq)]
pub struct Resource {
    /// The `resourceType` discriminator (e.g. `"Patient"`, `"Observation"`).
    pub resource_type: String,
    /// The resource `id`, if present.
    pub id: Option<String>,
    /// The `meta` object (version, profiles, tags, …), if present.
    pub meta: Option<Json>,
    /// The full resource JSON, for resource-specific field access.
    pub raw: Json,
}

impl Resource {
    /// Look up a top-level field by name from the raw JSON.
    pub fn field(&self, name: &str) -> Option<&Json> {
        self.raw.get(name)
    }
}

/// A single entry in a `Bundle`.
#[derive(Debug, Clone, PartialEq)]
pub struct BundleEntry {
    /// `fullUrl` identifying the entry.
    pub full_url: Option<String>,
    /// The contained resource.
    pub resource: Resource,
    /// `request` portion (for transaction/batch bundles), if present.
    pub request: Option<Json>,
    /// `response` portion (for transaction/batch response bundles), if present.
    pub response: Option<Json>,
}

/// A FHIR `Bundle` — an ordered collection of resources.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Bundle {
    /// Always `"Bundle"`.
    pub resource_type: String,
    /// Bundle `id`, if present.
    pub id: Option<String>,
    /// The entries, in document order.
    pub entry: Vec<BundleEntry>,
}

impl Bundle {
    /// Total number of entries.
    pub fn len(&self) -> usize {
        self.entry.len()
    }
    /// Whether the bundle has no entries.
    pub fn is_empty(&self) -> bool {
        self.entry.is_empty()
    }
}

/// A parsed FHIR document: either a `Bundle` or a single resource.
#[derive(Debug, Clone, PartialEq)]
pub enum Fhir {
    /// A bundle of resources.
    Bundle(Bundle),
    /// A single resource.
    Resource(Resource),
}
