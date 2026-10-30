//! Relationship graph construction over a parsed [`IfcFile`].
//!
//! IFC models building data as a graph of entity instances connected by
//! *relationship* entities. This module builds an index that lets callers
//! traverse the most important relationship types without re-walking the
//! instance list each time:
//!
//! - `IfcRelContainedInSpatialStructure` — which elements belong to a
//!   spatial container (storey, site, …).
//! - `IfcRelAssociatesMaterial` — which elements use which material.
//! - `IfcRelDefinesByProperties` — which elements carry which property set.
//! - `IfcRelAggregates` — the whole/part decomposition (e.g. site → building).
//!
//! The graph is a lightweight view over the file: it stores no data of its own
//! beyond the indices, and borrows the file for all lookups.

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

use crate::error::IfcError;
use crate::model::{EntityInstance, IfcFile, IfcValue};

/// A spatial containment link: `element_id` is contained in `structure_id`.
#[derive(Debug, Clone, PartialEq)]
pub struct Containment {
    /// The spatial structure instance id (e.g. an `IfcBuildingStorey`).
    pub structure_id: usize,
    /// The contained element instance id.
    pub element_id: usize,
}

/// A material association: `element_id` uses `material_id`.
#[derive(Debug, Clone, PartialEq)]
pub struct MaterialAssoc {
    /// The material definition instance id.
    pub material_id: usize,
    /// The element instance id.
    pub element_id: usize,
}

/// A property-set definition link: `element_id` is defined by `pset_id`.
#[derive(Debug, Clone, PartialEq)]
pub struct PropertyDef {
    /// The property set instance id (an `IfcPropertySet`).
    pub pset_id: usize,
    /// The element instance id.
    pub element_id: usize,
}

/// An aggregation (whole/part) link: `part_id` is part of `whole_id`.
#[derive(Debug, Clone, PartialEq)]
pub struct Aggregation {
    /// The whole instance id.
    pub whole_id: usize,
    /// The part instance id.
    pub part_id: usize,
}

/// A relationship graph built from an [`IfcFile`].
///
/// Construct with [`RelGraph::build`]. The graph indexes relationship instances
/// by their type and provides typed traversal helpers.
#[derive(Debug, Clone)]
pub struct RelGraph<'a> {
    file: &'a IfcFile,
    /// Maps a relationship type name to the indices of its instances.
    by_type: BTreeMap<&'a str, Vec<usize>>,
}

impl<'a> RelGraph<'a> {
    /// Build a relationship graph from a parsed file.
    ///
    /// The graph indexes every instance whose type name starts with
    /// `IFCREL` (the IFC convention for relationship entities).
    pub fn build(file: &'a IfcFile) -> Self {
        let mut by_type: BTreeMap<&str, Vec<usize>> = BTreeMap::new();
        for inst in &file.instances {
            if inst.type_name.starts_with("IFCREL") {
                by_type.entry(&inst.type_name).or_default().push(inst.id);
            }
        }
        Self { file, by_type }
    }

    /// Return the ids of all relationship instances of the given type.
    pub fn rel_ids(&self, type_name: &str) -> &[usize] {
        self.by_type.get(type_name).map(|v| v.as_slice()).unwrap_or(&[])
    }

    /// Return the relationship instance of `type_name` with the given id.
    pub fn rel(&self, type_name: &str, id: usize) -> Option<&EntityInstance> {
        self.rel_ids(type_name)
            .iter()
            .find(|&&rid| rid == id)
            .and_then(|rid| self.file.instance(*rid))
    }

    /// Iterate over all relationship instances of a given type.
    pub fn rels(&self, type_name: &str) -> RelIter<'a> {
        let ids: Vec<usize> = self.rel_ids(type_name).to_vec();
        RelIter { file: self.file, ids, index: 0 }
    }

    // -----------------------------------------------------------------------
    // Spatial containment: IfcRelContainedInSpatialStructure
    // -----------------------------------------------------------------------

    /// Return all spatial containment links in the file.
    ///
    /// `IfcRelContainedInSpatialStructure` has the form:
    /// `(GlobalId, OwnerHistory, Name, Description, RelatedObjects, RelatingStructure)`.
    pub fn containment(&self) -> Result<Vec<Containment>, IfcError> {
        let mut out = Vec::new();
        for rel in self.rels("IFCRELCONTAINEDINSPATIALSTRUCTURE") {
            let objects = rel.arg(4).and_then(IfcValue::as_list).ok_or_else(|| {
                IfcError::relationship(
                    "IfcRelContainedInSpatialStructure: RelatedObjects (arg 4) must be a list",
                )
            })?;
            let structure = rel
                .arg(5)
                .and_then(IfcValue::as_ref)
                .ok_or_else(|| IfcError::relationship("IfcRelContainedInSpatialStructure: RelatingStructure (arg 5) must be a reference"))?;
            for obj in objects {
                let elem = obj.as_ref().ok_or_else(|| {
                    IfcError::relationship("IfcRelContainedInSpatialStructure: RelatedObjects item must be a reference")
                })?;
                out.push(Containment { structure_id: structure, element_id: elem });
            }
        }
        Ok(out)
    }

    /// Return the ids of all elements contained in the given spatial structure.
    pub fn contained_elements(&self, structure_id: usize) -> Result<Vec<usize>, IfcError> {
        Ok(self
            .containment()?
            .into_iter()
            .filter(|c| c.structure_id == structure_id)
            .map(|c| c.element_id)
            .collect())
    }

    // -----------------------------------------------------------------------
    // Material association: IfcRelAssociatesMaterial
    // -----------------------------------------------------------------------

    /// Return all material associations in the file.
    ///
    /// `IfcRelAssociatesMaterial` has the form:
    /// `(GlobalId, OwnerHistory, Name, Description, RelatedObjects, RelatingMaterial)`.
    pub fn material_associations(&self) -> Result<Vec<MaterialAssoc>, IfcError> {
        let mut out = Vec::new();
        for rel in self.rels("IFCRELASSOCIATESMATERIAL") {
            let objects = rel.arg(4).and_then(IfcValue::as_list).ok_or_else(|| {
                IfcError::relationship(
                    "IfcRelAssociatesMaterial: RelatedObjects (arg 4) must be a list",
                )
            })?;
            let material = rel.arg(5).and_then(IfcValue::as_ref).ok_or_else(|| {
                IfcError::relationship(
                    "IfcRelAssociatesMaterial: RelatingMaterial (arg 5) must be a reference",
                )
            })?;
            for obj in objects {
                let elem = obj.as_ref().ok_or_else(|| {
                    IfcError::relationship(
                        "IfcRelAssociatesMaterial: RelatedObjects item must be a reference",
                    )
                })?;
                out.push(MaterialAssoc { material_id: material, element_id: elem });
            }
        }
        Ok(out)
    }

    // -----------------------------------------------------------------------
    // Property set definition: IfcRelDefinesByProperties
    // -----------------------------------------------------------------------

    /// Return all property-set definition links in the file.
    ///
    /// `IfcRelDefinesByProperties` has the form:
    /// `(GlobalId, OwnerHistory, Name, Description, RelatedObjects, RelatingDefinition)`.
    pub fn property_definitions(&self) -> Result<Vec<PropertyDef>, IfcError> {
        let mut out = Vec::new();
        for rel in self.rels("IFCRELDEFINESBYPROPERTIES") {
            let objects = rel.arg(4).and_then(IfcValue::as_list).ok_or_else(|| {
                IfcError::relationship(
                    "IfcRelDefinesByProperties: RelatedObjects (arg 4) must be a list",
                )
            })?;
            let pset = rel.arg(5).and_then(IfcValue::as_ref).ok_or_else(|| {
                IfcError::relationship(
                    "IfcRelDefinesByProperties: RelatingDefinition (arg 5) must be a reference",
                )
            })?;
            for obj in objects {
                let elem = obj.as_ref().ok_or_else(|| {
                    IfcError::relationship(
                        "IfcRelDefinesByProperties: RelatedObjects item must be a reference",
                    )
                })?;
                out.push(PropertyDef { pset_id: pset, element_id: elem });
            }
        }
        Ok(out)
    }

    // -----------------------------------------------------------------------
    // Aggregation: IfcRelAggregates
    // -----------------------------------------------------------------------

    /// Return all aggregation links in the file.
    ///
    /// `IfcRelAggregates` has the form:
    /// `(GlobalId, OwnerHistory, Name, Description, RelatingObject, RelatedObjects)`.
    pub fn aggregations(&self) -> Result<Vec<Aggregation>, IfcError> {
        let mut out = Vec::new();
        for rel in self.rels("IFCRELAGGREGATES") {
            let whole = rel.arg(4).and_then(IfcValue::as_ref).ok_or_else(|| {
                IfcError::relationship(
                    "IfcRelAggregates: RelatingObject (arg 4) must be a reference",
                )
            })?;
            let objects = rel.arg(5).and_then(IfcValue::as_list).ok_or_else(|| {
                IfcError::relationship("IfcRelAggregates: RelatedObjects (arg 5) must be a list")
            })?;
            for obj in objects {
                let part = obj.as_ref().ok_or_else(|| {
                    IfcError::relationship(
                        "IfcRelAggregates: RelatedObjects item must be a reference",
                    )
                })?;
                out.push(Aggregation { whole_id: whole, part_id: part });
            }
        }
        Ok(out)
    }

    /// Return the ids of all parts aggregated into `whole_id`.
    pub fn parts_of(&self, whole_id: usize) -> Result<Vec<usize>, IfcError> {
        Ok(self
            .aggregations()?
            .into_iter()
            .filter(|a| a.whole_id == whole_id)
            .map(|a| a.part_id)
            .collect())
    }

    /// Return the id of the whole that `part_id` belongs to, if any.
    pub fn whole_of(&self, part_id: usize) -> Result<Option<usize>, IfcError> {
        Ok(self.aggregations()?.into_iter().find(|a| a.part_id == part_id).map(|a| a.whole_id))
    }

    /// Return the human-readable type name of an instance, if it exists.
    pub fn type_of(&self, id: usize) -> Option<&str> {
        self.file.instance(id).map(|i| i.type_name.as_str())
    }

    /// Return the string `Name` argument of an instance (arg 2 in most roots).
    pub fn name_of(&self, id: usize) -> Option<&str> {
        self.file.instance(id).and_then(|i| i.arg(2)).and_then(IfcValue::as_string)
    }

    /// Return the raw file reference.
    pub fn file(&self) -> &'a IfcFile {
        self.file
    }
}

/// An iterator over relationship instances of a given type.
#[derive(Debug, Clone)]
pub struct RelIter<'a> {
    file: &'a IfcFile,
    ids: Vec<usize>,
    index: usize,
}

impl<'a> Iterator for RelIter<'a> {
    type Item = &'a EntityInstance;

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.ids.len().saturating_sub(self.index);
        (remaining, Some(remaining))
    }

    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.ids.len() {
            return None;
        }
        let id = self.ids[self.index];
        self.index += 1;
        self.file.instance(id)
    }
}

impl ExactSizeIterator for RelIter<'_> {}

/// Helper to extract a list of references from an [`IfcValue`].
pub fn as_ref_list(value: &IfcValue) -> Option<Vec<usize>> {
    value.as_list().map(|items| items.iter().filter_map(IfcValue::as_ref).collect())
}

/// Helper to extract a list of strings from an [`IfcValue`].
pub fn as_string_list(value: &IfcValue) -> Option<Vec<&str>> {
    value.as_list().map(|items| items.iter().filter_map(IfcValue::as_string).collect())
}

/// Helper to extract a list of f64 values from an [`IfcValue`].
pub fn as_real_list(value: &IfcValue) -> Option<Vec<f64>> {
    value.as_list().map(|items| items.iter().filter_map(IfcValue::as_real).collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(not(feature = "std"))]
    use alloc::string::ToString;
    use crate::model::IfcHeader;

    fn sample_file() -> IfcFile {
        let instances = vec![
            // IfcProject #1
            EntityInstance::new(
                1,
                "IFCPROJECT",
                vec![
                    IfcValue::String("proj-guid".to_string()),
                    IfcValue::Derived,
                    IfcValue::String("Project".to_string()),
                    IfcValue::Derived,
                    IfcValue::Derived,
                    IfcValue::Derived,
                    IfcValue::List(vec![IfcValue::Ref(10)]),
                    IfcValue::Derived,
                ],
            ),
            // IfcSite #2
            EntityInstance::new(
                2,
                "IFCSITE",
                vec![
                    IfcValue::String("site-guid".to_string()),
                    IfcValue::Derived,
                    IfcValue::String("Site".to_string()),
                ],
            ),
            // IfcBuilding #3
            EntityInstance::new(
                3,
                "IFCBUILDING",
                vec![
                    IfcValue::String("bldg-guid".to_string()),
                    IfcValue::Derived,
                    IfcValue::String("Building".to_string()),
                ],
            ),
            // IfcBuildingStorey #4
            EntityInstance::new(
                4,
                "IFCBUILDINGSTOREY",
                vec![
                    IfcValue::String("storey-guid".to_string()),
                    IfcValue::Derived,
                    IfcValue::String("Storey".to_string()),
                ],
            ),
            // IfcWall #5
            EntityInstance::new(
                5,
                "IFCWALL",
                vec![
                    IfcValue::String("wall-guid".to_string()),
                    IfcValue::Derived,
                    IfcValue::String("Wall".to_string()),
                ],
            ),
            // IfcRelAggregates: project(1) → site(2)
            EntityInstance::new(
                6,
                "IFCRELAGGREGATES",
                vec![
                    IfcValue::Derived,
                    IfcValue::Derived,
                    IfcValue::Derived,
                    IfcValue::Derived,
                    IfcValue::Ref(1),
                    IfcValue::List(vec![IfcValue::Ref(2)]),
                ],
            ),
            // IfcRelAggregates: site(2) → building(3)
            EntityInstance::new(
                7,
                "IFCRELAGGREGATES",
                vec![
                    IfcValue::Derived,
                    IfcValue::Derived,
                    IfcValue::Derived,
                    IfcValue::Derived,
                    IfcValue::Ref(2),
                    IfcValue::List(vec![IfcValue::Ref(3)]),
                ],
            ),
            // IfcRelAggregates: building(3) → storey(4)
            EntityInstance::new(
                8,
                "IFCRELAGGREGATES",
                vec![
                    IfcValue::Derived,
                    IfcValue::Derived,
                    IfcValue::Derived,
                    IfcValue::Derived,
                    IfcValue::Ref(3),
                    IfcValue::List(vec![IfcValue::Ref(4)]),
                ],
            ),
            // IfcRelContainedInSpatialStructure: storey(4) contains wall(5)
            EntityInstance::new(
                9,
                "IFCRELCONTAINEDINSPATIALSTRUCTURE",
                vec![
                    IfcValue::Derived,
                    IfcValue::Derived,
                    IfcValue::Derived,
                    IfcValue::Derived,
                    IfcValue::List(vec![IfcValue::Ref(5)]),
                    IfcValue::Ref(4),
                ],
            ),
            // IfcGeometricRepresentationContext #10
            EntityInstance::new(
                10,
                "IFCGEOMETRICREPRESENTATIONCONTEXT",
                vec![
                    IfcValue::Derived,
                    IfcValue::String("Model".to_string()),
                    IfcValue::Real(3.0),
                    IfcValue::Derived,
                    IfcValue::Derived,
                    IfcValue::Derived,
                ],
            ),
        ];
        IfcFile { header: IfcHeader::default(), instances }
    }

    #[test]
    fn builds_graph_and_traverses() {
        let file = sample_file();
        let graph = RelGraph::build(&file);

        let agg = graph.aggregations().unwrap();
        assert_eq!(agg.len(), 3);
        assert_eq!(graph.parts_of(1).unwrap(), vec![2]);
        assert_eq!(graph.parts_of(2).unwrap(), vec![3]);
        assert_eq!(graph.parts_of(3).unwrap(), vec![4]);
        assert_eq!(graph.whole_of(5).unwrap(), None);

        let cont = graph.containment().unwrap();
        assert_eq!(cont.len(), 1);
        assert_eq!(cont[0].structure_id, 4);
        assert_eq!(cont[0].element_id, 5);
        assert_eq!(graph.contained_elements(4).unwrap(), vec![5]);
    }

    #[test]
    fn material_and_property_rels_empty_when_absent() {
        let file = sample_file();
        let graph = RelGraph::build(&file);
        assert!(graph.material_associations().unwrap().is_empty());
        assert!(graph.property_definitions().unwrap().is_empty());
    }

    #[test]
    fn indexes_by_type() {
        let file = sample_file();
        let graph = RelGraph::build(&file);
        assert_eq!(graph.rel_ids("IFCRELAGGREGATES").len(), 3);
        assert_eq!(graph.rel_ids("IFCRELCONTAINEDINSPATIALSTRUCTURE").len(), 1);
        assert!(graph.rel_ids("IFCRELASSOCIATESMATERIAL").is_empty());
    }

    #[test]
    fn rel_iter() {
        let file = sample_file();
        let graph = RelGraph::build(&file);
        let count = graph.rels("IFCRELAGGREGATES").count();
        assert_eq!(count, 3);
    }
}
