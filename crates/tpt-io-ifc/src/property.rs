//! Property set traversal over a parsed [`IfcFile`].
//!
//! IFC carries building-element metadata in *property sets* (`IfcPropertySet`)
//! linked to elements via `IfcRelDefinesByProperties`. Each set holds named
//! properties — single values, bounded values, lists, or references — each
//! optionally carrying a unit. This module walks that chain:
//!
//! ```text
//! element ──IfcRelDefinesByProperties──▶ IfcPropertySet
//!                                            │
//!                                  IfcPropertySingleValue (name, value, unit)
//! ```

use alloc::string::String;
use alloc::vec::Vec;

use crate::error::IfcError;
use crate::graph::RelGraph;
use crate::model::{EntityInstance, IfcFile, IfcValue};

/// A resolved property value with its name and optional unit.
#[derive(Debug, Clone, PartialEq)]
pub struct Property<'a> {
    /// The property name.
    pub name: String,
    /// The resolved value.
    pub value: PropertyValue<'a>,
    /// The unit, if any.
    pub unit: Option<&'a str>,
}

/// The value of a resolved property.
#[derive(Debug, Clone, PartialEq)]
pub enum PropertyValue<'a> {
    /// A single literal value.
    Single(&'a IfcValue),
    /// A bounded value (lower/upper).
    Bounded {
        /// The lower bound.
        lower: &'a IfcValue,
        /// The upper bound.
        upper: &'a IfcValue,
        /// The setpoint, if any.
        setpoint: Option<&'a IfcValue>,
    },
    /// A list of values.
    List(Vec<&'a IfcValue>),
    /// A reference to a complex (table) value.
    Reference(&'a IfcValue),
    /// A textual value.
    Text(&'a str),
}

/// A resolved property set with its name and properties.
#[derive(Debug, Clone, PartialEq)]
pub struct PropertySet<'a> {
    /// The property set name (e.g. `Pset_WallCommon`).
    pub name: String,
    /// The properties in the set.
    pub properties: Vec<Property<'a>>,
}

/// Traverses property sets linked to elements in a parsed file.
///
/// Build with [`PropertyTraversal::new`], then query by element id or resolve
/// every property set in the file.
#[derive(Debug, Clone)]
pub struct PropertyTraversal<'a> {
    file: &'a IfcFile,
    graph: RelGraph<'a>,
}

impl<'a> PropertyTraversal<'a> {
    /// Create a new traversal over `file`.
    pub fn new(file: &'a IfcFile) -> Self {
        Self { file, graph: RelGraph::build(file) }
    }

    /// Return every property set linked to `element_id`, resolved.
    pub fn property_sets(&self, element_id: usize) -> Result<Vec<PropertySet<'a>>, IfcError> {
        let defs = self.graph.property_definitions()?;
        let mut out = Vec::new();
        for def in &defs {
            if def.element_id == element_id {
                if let Some(pset) = self.file.instance(def.pset_id) {
                    out.push(self.resolve_pset(pset)?);
                }
            }
        }
        Ok(out)
    }

    /// Return every property set in the file (across all elements).
    pub fn all_property_sets(&self) -> Result<Vec<(usize, PropertySet<'a>)>, IfcError> {
        let defs = self.graph.property_definitions()?;
        let mut out = Vec::new();
        for def in &defs {
            if let Some(pset) = self.file.instance(def.pset_id) {
                out.push((def.element_id, self.resolve_pset(pset)?));
            }
        }
        Ok(out)
    }

    /// Resolve a single `IfcPropertySet` instance into a [`PropertySet`].
    ///
    /// `IfcPropertySet` has the form:
    /// `(GlobalId, OwnerHistory, Name, Description, HasProperties)`.
    pub fn resolve_pset(&self, pset: &'a EntityInstance) -> Result<PropertySet<'a>, IfcError> {
        let name = pset.arg(2).and_then(IfcValue::as_string).unwrap_or("").to_string();
        let props_list = pset.arg(4).and_then(IfcValue::as_list).ok_or_else(|| {
            IfcError::relationship("IfcPropertySet: HasProperties (arg 4) must be a list")
        })?;

        let mut properties = Vec::with_capacity(props_list.len());
        for prop_val in props_list {
            let prop_id = prop_val.as_ref().ok_or_else(|| {
                IfcError::relationship("IfcPropertySet: HasProperties item must be a reference")
            })?;
            if let Some(prop_inst) = self.file.instance(prop_id) {
                properties.push(self.resolve_property(prop_inst)?);
            }
        }
        Ok(PropertySet { name, properties })
    }

    /// Resolve a single property instance.
    fn resolve_property(&self, inst: &'a EntityInstance) -> Result<Property<'a>, IfcError> {
        let name = inst.arg(0).and_then(IfcValue::as_string).unwrap_or("").to_string();

        match inst.type_name.as_str() {
            "IFCPROPERTYSINGLEVALUE" => self.resolve_single_value(inst, name),
            "IFCPROPERTYBOUNDEDVALUE" => self.resolve_bounded_value(inst, name),
            "IFCPROPERTYLISTVALUE" => self.resolve_list_value(inst, name),
            "IFCPROPERTYREFERENCEVALUE" => self.resolve_reference_value(inst, name),
            "IFCPROPERTYTABLEVALUE" => self.resolve_table_value(inst, name),
            _ => {
                // Unknown property type: surface as a single value if possible.
                let value = if let Some(v) = inst.arg(2) {
                    PropertyValue::Single(v)
                } else {
                    PropertyValue::Text("")
                };
                Ok(Property { name, value, unit: None })
            }
        }
    }

    /// `IfcPropertySingleValue` = (Name, Description, NominalValue, Unit).
    fn resolve_single_value(
        &self,
        inst: &'a EntityInstance,
        name: String,
    ) -> Result<Property<'a>, IfcError> {
        let value = inst.arg(2).ok_or_else(|| {
            IfcError::relationship("IfcPropertySingleValue: NominalValue (arg 2) required")
        })?;
        let unit = inst.arg(3).and_then(IfcValue::as_string);
        Ok(Property { name, value: PropertyValue::Single(value), unit })
    }

    /// `IfcPropertyBoundedValue` = (Name, Description, UpperBoundValue, LowerBoundValue, Unit).
    fn resolve_bounded_value(
        &self,
        inst: &'a EntityInstance,
        name: String,
    ) -> Result<Property<'a>, IfcError> {
        let upper = inst.arg(2).ok_or_else(|| {
            IfcError::relationship("IfcPropertyBoundedValue: UpperBoundValue (arg 2) required")
        })?;
        let lower = inst.arg(3).ok_or_else(|| {
            IfcError::relationship("IfcPropertyBoundedValue: LowerBoundValue (arg 3) required")
        })?;
        let setpoint = inst.arg(5).and_then(IfcValue::as_real).and_then(|_| inst.arg(5));
        let unit = inst.arg(4).and_then(IfcValue::as_string);
        Ok(Property { name, value: PropertyValue::Bounded { lower, upper, setpoint }, unit })
    }

    /// `IfcPropertyListValue` = (Name, Description, ListValues, Unit).
    fn resolve_list_value(
        &self,
        inst: &'a EntityInstance,
        name: String,
    ) -> Result<Property<'a>, IfcError> {
        let list = inst.arg(2).and_then(IfcValue::as_list).ok_or_else(|| {
            IfcError::relationship("IfcPropertyListValue: ListValues (arg 2) required")
        })?;
        let refs: Vec<&IfcValue> = list.iter().collect();
        let unit = inst.arg(3).and_then(IfcValue::as_string);
        Ok(Property { name, value: PropertyValue::List(refs), unit })
    }

    /// `IfcPropertyReferenceValue` = (Name, Description, UsageName, PropertyReferenceValue, Unit).
    fn resolve_reference_value(
        &self,
        inst: &'a EntityInstance,
        name: String,
    ) -> Result<Property<'a>, IfcError> {
        let value = inst.arg(3).ok_or_else(|| {
            IfcError::relationship(
                "IfcPropertyReferenceValue: PropertyReferenceValue (arg 3) required",
            )
        })?;
        let unit = inst.arg(4).and_then(IfcValue::as_string);
        Ok(Property { name, value: PropertyValue::Reference(value), unit })
    }

    /// `IfcPropertyTableValue` = (Name, Description, DefiningValues, DefinedValues, Expression, Unit).
    fn resolve_table_value(
        &self,
        inst: &'a EntityInstance,
        name: String,
    ) -> Result<Property<'a>, IfcError> {
        let defined = inst.arg(3).and_then(IfcValue::as_list);
        let unit = inst.arg(5).and_then(IfcValue::as_string);
        // Surface as a list of the defined values (the "output" column).
        let refs: Vec<&IfcValue> = defined.map(|d| d.iter().collect()).unwrap_or_default();
        Ok(Property { name, value: PropertyValue::List(refs), unit })
    }

    /// Return the raw file reference.
    pub fn file(&self) -> &'a IfcFile {
        self.file
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::IfcHeader;

    fn sample_file() -> IfcFile {
        // #1 IfcWall, #2 IfcRelDefinesByProperties, #3 IfcPropertySet,
        // #4 IfcPropertySingleValue, #5 IfcPropertyBoundedValue
        let instances = vec![
            EntityInstance::new(
                1,
                "IFCWALL",
                vec![
                    IfcValue::String("wall-guid".to_string()),
                    IfcValue::Derived,
                    IfcValue::String("Wall".to_string()),
                ],
            ),
            EntityInstance::new(
                2,
                "IFCRELDEFINESBYPROPERTIES",
                vec![
                    IfcValue::Derived,
                    IfcValue::Derived,
                    IfcValue::Derived,
                    IfcValue::Derived,
                    IfcValue::List(vec![IfcValue::Ref(1)]),
                    IfcValue::Ref(3),
                ],
            ),
            EntityInstance::new(
                3,
                "IFCPROPERTYSET",
                vec![
                    IfcValue::String("pset-guid".to_string()),
                    IfcValue::Derived,
                    IfcValue::String("Pset_WallCommon".to_string()),
                    IfcValue::Derived,
                    IfcValue::List(vec![IfcValue::Ref(4), IfcValue::Ref(5)]),
                ],
            ),
            EntityInstance::new(
                4,
                "IFCPROPERTYSINGLEVALUE",
                vec![
                    IfcValue::String("IsExternal".to_string()),
                    IfcValue::Derived,
                    IfcValue::Bool(true),
                    IfcValue::Derived,
                ],
            ),
            EntityInstance::new(
                5,
                "IFCPROPERTYBOUNDEDVALUE",
                vec![
                    IfcValue::String("ThermalTransmittance".to_string()),
                    IfcValue::Derived,
                    IfcValue::Real(0.5),
                    IfcValue::Real(0.1),
                    IfcValue::Derived,
                    IfcValue::Derived,
                ],
            ),
        ];
        IfcFile { header: IfcHeader::default(), instances }
    }

    #[test]
    fn resolves_property_sets_for_element() {
        let file = sample_file();
        let trav = PropertyTraversal::new(&file);
        let psets = trav.property_sets(1).unwrap();
        assert_eq!(psets.len(), 1);
        assert_eq!(psets[0].name, "Pset_WallCommon");
        assert_eq!(psets[0].properties.len(), 2);

        let p0 = &psets[0].properties[0];
        assert_eq!(p0.name, "IsExternal");
        assert!(matches!(p0.value, PropertyValue::Single(IfcValue::Bool(true))));

        let p1 = &psets[0].properties[1];
        assert_eq!(p1.name, "ThermalTransmittance");
        assert!(matches!(p1.value, PropertyValue::Bounded { .. }));
    }

    #[test]
    fn all_property_sets() {
        let file = sample_file();
        let trav = PropertyTraversal::new(&file);
        let all = trav.all_property_sets().unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].0, 1);
    }

    #[test]
    fn no_property_sets_for_unlinked_element() {
        let file = sample_file();
        let trav = PropertyTraversal::new(&file);
        let psets = trav.property_sets(99).unwrap();
        assert!(psets.is_empty());
    }
}
