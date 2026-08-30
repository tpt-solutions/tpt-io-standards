//! Spatial structure navigation (Site → Building → Storey → Space).
//!
//! IFC organizes a building's spatial hierarchy via aggregation
//! (`IfcRelAggregates`) and containment
//! (`IfcRelContainedInSpatialStructure`) relationships. This module walks
//! that hierarchy and exposes it as a tree rooted at the project:
//!
//! ```text
//! IfcProject
//! └─ IfcSite
//!    └─ IfcBuilding
//!       └─ IfcBuildingStorey
//!          └─ IfcSpace
//!             └─ IfcElement (walls, slabs, …)
//! ```

use alloc::string::String;
use alloc::vec::Vec;

use crate::error::IfcError;
use crate::graph::RelGraph;
use crate::model::{EntityInstance, IfcFile, IfcValue};

/// A node in the spatial hierarchy tree.
#[derive(Debug, Clone, PartialEq)]
pub struct SpatialNode {
    /// The instance id of this spatial element.
    pub id: usize,
    /// The entity type name, e.g. `IFCBUILDINGSTOREY`.
    pub type_name: String,
    /// The human-readable name (arg 2), if any.
    pub name: String,
    /// The child spatial nodes.
    pub children: Vec<SpatialNode>,
}

impl SpatialNode {
    /// Return the number of direct children.
    pub fn child_count(&self) -> usize {
        self.children.len()
    }

    /// Return `true` if this node has no children.
    pub fn is_leaf(&self) -> bool {
        self.children.is_empty()
    }
}

/// A spatial structure navigator over a parsed IFC file.
///
/// Build with [`SpatialNavigator::new`], then call [`SpatialNavigator::project_tree`] to obtain
/// the full hierarchy, or use the lower-level helpers to walk specific
/// subtrees.
#[derive(Debug, Clone)]
pub struct SpatialNavigator<'a> {
    file: &'a IfcFile,
    graph: RelGraph<'a>,
}

impl<'a> SpatialNavigator<'a> {
    /// Create a new navigator over `file`.
    pub fn new(file: &'a IfcFile) -> Self {
        Self { file, graph: RelGraph::build(file) }
    }

    /// Return the full spatial hierarchy rooted at the project.
    ///
    /// If the file contains more than one project, a tree is returned for
    /// each. The project is identified by its `IFCPROJECT` type.
    pub fn project_tree(&self) -> Result<Vec<SpatialNode>, IfcError> {
        let mut roots = Vec::new();
        for inst in &self.file.instances {
            if inst.type_name == "IFCPROJECT" {
                roots.push(self.build_node(inst, &["IFCSITE", "IFCBUILDING", "IFCBUILDINGSTOREY", "IFCSPACE"])?);
            }
        }
        Ok(roots)
    }

    /// Return the spatial hierarchy rooted at a specific site.
    pub fn site_tree(&self, site_id: usize) -> Result<SpatialNode, IfcError> {
        let site = self.file.instance(site_id).ok_or_else(|| IfcError::unknown_instance(0, site_id))?;
        self.build_node(site, &["IFCBUILDING", "IFCBUILDINGSTOREY", "IFCSPACE"])
    }

    /// Return the spatial hierarchy rooted at a specific building.
    pub fn building_tree(&self, building_id: usize) -> Result<SpatialNode, IfcError> {
        let bldg = self
            .file
            .instance(building_id)
            .ok_or_else(|| IfcError::unknown_instance(0, building_id))?;
        self.build_node(bldg, &["IFCBUILDINGSTOREY", "IFCSPACE"])
    }

    /// Return the spatial hierarchy rooted at a specific storey.
    pub fn storey_tree(&self, storey_id: usize) -> Result<SpatialNode, IfcError> {
        let storey = self
            .file
            .instance(storey_id)
            .ok_or_else(|| IfcError::unknown_instance(0, storey_id))?;
        self.build_node(storey, &["IFCSPACE"])
    }

    /// Return the ids of all elements contained in a spatial structure.
    pub fn elements_in(&self, structure_id: usize) -> Result<Vec<usize>, IfcError> {
        self.graph.contained_elements(structure_id)
    }

    /// Return the id of the spatial structure that contains `element_id`, if any.
    ///
    /// Walks the containment relationships to find the direct container.
    pub fn container_of(&self, element_id: usize) -> Result<Option<usize>, IfcError> {
        for c in self.graph.containment()? {
            if c.element_id == element_id {
                return Ok(Some(c.structure_id));
            }
        }
        Ok(None)
    }

    /// Return the id of the storey that contains `element_id`, if any.
    ///
    /// This is a convenience that returns the container only when it is a
    /// storey (`IFCBUILDINGSTOREY`).
    pub fn storey_of(&self, element_id: usize) -> Result<Option<usize>, IfcError> {
        if let Some(container_id) = self.container_of(element_id)? {
            if let Some(container) = self.file.instance(container_id) {
                if container.type_name == "IFCBUILDINGSTOREY" {
                    return Ok(Some(container_id));
                }
            }
        }
        Ok(None)
    }

    /// Build a spatial node for `instance`, recursing into children whose type
    /// names appear in `child_types`.
    fn build_node(
        &self,
        instance: &'a EntityInstance,
        child_types: &[&str],
    ) -> Result<SpatialNode, IfcError> {
        let id = instance.id;
        let type_name = instance.type_name.clone();
        let name = instance
            .arg(2)
            .and_then(IfcValue::as_string)
            .unwrap_or("")
            .to_string();

        // Find children via aggregation (whole → parts).
        let mut children = Vec::new();
        if let Ok(parts) = self.graph.parts_of(id) {
            for part_id in parts {
                if let Some(part) = self.file.instance(part_id) {
                    if child_types.iter().any(|t| part.type_name == *t) {
                        let next_types = next_child_types(child_types, &part.type_name);
                        children.push(self.build_node(part, next_types)?);
                    }
                }
            }
        }

        Ok(SpatialNode { id, type_name, name, children })
    }

    /// Return the raw file reference.
    pub fn file(&self) -> &'a IfcFile {
        self.file
    }
}

/// Given the current list of allowed child types and the type of a matched
/// child, return the list of types allowed for the *next* level down.
fn next_child_types<'b>(child_types: &'b [&'b str], matched: &str) -> &'b [&'b str] {
    // Find the position of the matched type and return everything after it.
    for (i, t) in child_types.iter().enumerate() {
        if *t == matched {
            return &child_types[i + 1..];
        }
    }
    &[]
}

/// Return the ids of all instances of a given type in the file.
pub fn find_by_type(file: &IfcFile, type_name: &str) -> Vec<usize> {
    file.instances_of(type_name).map(|i| i.id).collect()
}

/// Return the human-readable name of an instance (arg 2), if any.
pub fn name_of(file: &IfcFile, id: usize) -> Option<&str> {
    file.instance(id).and_then(|i| i.arg(2)).and_then(IfcValue::as_string)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::IfcHeader;

    /// Build a sample file with a full spatial hierarchy:
    /// Project #1 → Site #2 → Building #3 → Storey #4 → Space #5,
    /// and Wall #6 contained in Storey #4.
    fn sample_file() -> IfcFile {
        let instances = vec![
            EntityInstance::new(1, "IFCPROJECT", vec![
                IfcValue::String("proj".to_string()),
                IfcValue::Derived,
                IfcValue::String("Project".to_string()),
            ]),
            EntityInstance::new(2, "IFCSITE", vec![
                IfcValue::String("site".to_string()),
                IfcValue::Derived,
                IfcValue::String("My Site".to_string()),
            ]),
            EntityInstance::new(3, "IFCBUILDING", vec![
                IfcValue::String("bldg".to_string()),
                IfcValue::Derived,
                IfcValue::String("Building A".to_string()),
            ]),
            EntityInstance::new(4, "IFCBUILDINGSTOREY", vec![
                IfcValue::String("storey".to_string()),
                IfcValue::Derived,
                IfcValue::String("Ground Floor".to_string()),
            ]),
            EntityInstance::new(5, "IFCSPACE", vec![
                IfcValue::String("space".to_string()),
                IfcValue::Derived,
                IfcValue::String("Room 101".to_string()),
            ]),
            EntityInstance::new(6, "IFCWALL", vec![
                IfcValue::String("wall".to_string()),
                IfcValue::Derived,
                IfcValue::String("Wall-1".to_string()),
            ]),
            // Project → Site
            EntityInstance::new(10, "IFCRELAGGREGATES", vec![
                IfcValue::Derived,
                IfcValue::Derived,
                IfcValue::Derived,
                IfcValue::Derived,
                IfcValue::Ref(1),
                IfcValue::List(vec![IfcValue::Ref(2)]),
            ]),
            // Site → Building
            EntityInstance::new(11, "IFCRELAGGREGATES", vec![
                IfcValue::Derived,
                IfcValue::Derived,
                IfcValue::Derived,
                IfcValue::Derived,
                IfcValue::Ref(2),
                IfcValue::List(vec![IfcValue::Ref(3)]),
            ]),
            // Building → Storey
            EntityInstance::new(12, "IFCRELAGGREGATES", vec![
                IfcValue::Derived,
                IfcValue::Derived,
                IfcValue::Derived,
                IfcValue::Derived,
                IfcValue::Ref(3),
                IfcValue::List(vec![IfcValue::Ref(4)]),
            ]),
            // Storey → Space
            EntityInstance::new(13, "IFCRELAGGREGATES", vec![
                IfcValue::Derived,
                IfcValue::Derived,
                IfcValue::Derived,
                IfcValue::Derived,
                IfcValue::Ref(4),
                IfcValue::List(vec![IfcValue::Ref(5)]),
            ]),
            // Storey contains Wall
            EntityInstance::new(14, "IFCRELCONTAINEDINSPATIALSTRUCTURE", vec![
                IfcValue::Derived,
                IfcValue::Derived,
                IfcValue::Derived,
                IfcValue::Derived,
                IfcValue::List(vec![IfcValue::Ref(6)]),
                IfcValue::Ref(4),
            ]),
        ];
        IfcFile { header: IfcHeader::default(), instances }
    }

    #[test]
    fn project_tree_traverses_full_hierarchy() {
        let file = sample_file();
        let nav = SpatialNavigator::new(&file);
        let trees = nav.project_tree().unwrap();
        assert_eq!(trees.len(), 1);
        let root = &trees[0];
        assert_eq!(root.id, 1);
        assert_eq!(root.type_name, "IFCPROJECT");
        assert_eq!(root.children.len(), 1); // Site

        let site = &root.children[0];
        assert_eq!(site.id, 2);
        assert_eq!(site.name, "My Site");
        assert_eq!(site.children.len(), 1); // Building

        let bldg = &site.children[0];
        assert_eq!(bldg.id, 3);
        assert_eq!(bldg.name, "Building A");
        assert_eq!(bldg.children.len(), 1); // Storey

        let storey = &bldg.children[0];
        assert_eq!(storey.id, 4);
        assert_eq!(storey.name, "Ground Floor");
        assert_eq!(storey.children.len(), 1); // Space

        let space = &storey.children[0];
        assert_eq!(space.id, 5);
        assert_eq!(space.name, "Room 101");
        assert!(space.is_leaf());
    }

    #[test]
    fn elements_in_and_container_of() {
        let file = sample_file();
        let nav = SpatialNavigator::new(&file);
        assert_eq!(nav.elements_in(4).unwrap(), vec![6]);
        assert_eq!(nav.container_of(6).unwrap(), Some(4));
        assert_eq!(nav.storey_of(6).unwrap(), Some(4));
        assert_eq!(nav.container_of(1).unwrap(), None);
    }

    #[test]
    fn storey_tree() {
        let file = sample_file();
        let nav = SpatialNavigator::new(&file);
        let tree = nav.storey_tree(4).unwrap();
        assert_eq!(tree.id, 4);
        assert_eq!(tree.children.len(), 1); // Space
        assert_eq!(tree.children[0].id, 5);
    }

    #[test]
    fn find_by_type_and_name() {
        let file = sample_file();
        let storeys = find_by_type(&file, "IFCBUILDINGSTOREY");
        assert_eq!(storeys, vec![4]);
        assert_eq!(name_of(&file, 4), Some("Ground Floor"));
        assert_eq!(name_of(&file, 99), None);
    }

    #[test]
    fn empty_file_yields_no_trees() {
        let file = IfcFile { header: IfcHeader::default(), instances: Vec::new() };
        let nav = SpatialNavigator::new(&file);
        assert!(nav.project_tree().unwrap().is_empty());
    }
}
