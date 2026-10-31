//! The OpenSCENARIO structural model.

use alloc::string::{String, ToString};
use alloc::vec::Vec;

use crate::error::OpenScenarioError;

/// The `<FileHeader>` element.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FileHeader {
    /// Major revision; OpenSCENARIO 1.x files use `1`.
    pub rev_major: u32,
    /// Minor revision, e.g. `1` for 1.1.
    pub rev_minor: u32,
    /// Creation date as written.
    pub date: String,
    /// Free-text description.
    pub description: String,
    /// Author name as written.
    pub author: String,
}

/// A `<ParameterDeclaration>`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Parameter {
    /// Parameter name, referenced as `$name` in the file.
    pub name: String,
    /// Declared type, e.g. `double`, `string`, `integer`.
    pub kind: String,
    /// Default value as written.
    pub value: String,
}

/// The kind of a scenario entity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EntityKind {
    /// A `<Vehicle>` entity.
    Vehicle,
    /// A `<Pedestrian>` entity.
    Pedestrian,
    /// A `<MiscObject>` entity.
    MiscObject,
    /// A `<ExternalObjectReference>` entity.
    ExternalReference,
    /// Anything else, kept by element name.
    Other(String),
}

/// A `<ScenarioObject>` and the entity it declares.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScenarioObject {
    /// Entity name, unique within the scenario.
    pub name: String,
    /// The entity's kind.
    pub kind: EntityKind,
    /// The entity's `vehicleCategory` or `pedestrianCategory`, when given.
    pub category: Option<String>,
}

/// An `<Act>` and the number of maneuver groups it contains.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Act {
    /// Act name.
    pub name: String,
    /// Number of `<ManeuverGroup>` children.
    pub maneuver_groups: usize,
}

/// A `<Story>` and its acts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Story {
    /// Story name.
    pub name: String,
    /// Acts in document order.
    pub acts: Vec<Act>,
}

/// A parsed OpenSCENARIO file.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct OpenScenario {
    /// The file header.
    pub header: FileHeader,
    /// Parameter declarations.
    pub parameters: Vec<Parameter>,
    /// Catalog directories as `"<CatalogName>:<path>"` strings.
    pub catalog_locations: Vec<String>,
    /// The road network's `LogicFile` path (an OpenDRIVE file), if declared.
    pub road_network: Option<String>,
    /// Scenario entities in document order.
    pub entities: Vec<ScenarioObject>,
    /// Stories of the storyboard in document order.
    pub stories: Vec<Story>,
}

impl OpenScenario {
    /// Check the rules a scenario runner depends on: OpenSCENARIO 1.x revision, unique entity
    /// names, and unique story names.
    pub fn validate(&self) -> Result<(), OpenScenarioError> {
        if self.header.rev_major != 1 {
            return Err(OpenScenarioError::Invalid(
                "only OpenSCENARIO 1.x files are supported".to_string(),
            ));
        }
        let mut names: Vec<&str> = self.entities.iter().map(|e| e.name.as_str()).collect();
        names.sort_unstable();
        if names.windows(2).any(|w| w[0] == w[1]) {
            return Err(OpenScenarioError::Invalid("duplicate entity name".to_string()));
        }
        let mut stories: Vec<&str> = self.stories.iter().map(|s| s.name.as_str()).collect();
        stories.sort_unstable();
        if stories.windows(2).any(|w| w[0] == w[1]) {
            return Err(OpenScenarioError::Invalid("duplicate story name".to_string()));
        }
        Ok(())
    }
}
