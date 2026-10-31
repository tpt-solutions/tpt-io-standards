//! The AAS environment model.

use alloc::string::{String, ToString};
use alloc::vec::Vec;

use crate::error::AasError;

/// A submodel element: its id-short and its model type (`Property`, `SubmodelElementCollection`, …).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubmodelElement {
    /// The element's `idShort`, if given.
    pub id_short: Option<String>,
    /// The `modelType`, e.g. `Property`.
    pub model_type: String,
}

/// A submodel and its elements.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Submodel {
    /// The submodel identifier.
    pub id: String,
    /// The submodel's `idShort`, if given.
    pub id_short: Option<String>,
    /// Elements in document order.
    pub elements: Vec<SubmodelElement>,
}

/// An asset administration shell.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Shell {
    /// The shell identifier.
    pub id: String,
    /// The shell's `idShort`, if given.
    pub id_short: Option<String>,
    /// The asset's `globalAssetId`, if given.
    pub global_asset_id: Option<String>,
    /// The identifiers of the submodels the shell refers to (the last key of each reference).
    pub submodel_refs: Vec<String>,
}

/// A whole environment.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Environment {
    /// Shells in document order.
    pub shells: Vec<Shell>,
    /// Submodels in document order.
    pub submodels: Vec<Submodel>,
    /// The number of concept descriptions (their contents are not read).
    pub concept_descriptions: usize,
}

impl Environment {
    /// Check that shell ids and submodel ids are each unique. References to submodels outside
    /// the environment are allowed, since AAS references may be external.
    pub fn validate(&self) -> Result<(), AasError> {
        let mut shells: Vec<&str> = self.shells.iter().map(|s| s.id.as_str()).collect();
        shells.sort_unstable();
        if shells.windows(2).any(|w| w[0] == w[1]) {
            return Err(AasError::Invalid("duplicate shell id".to_string()));
        }
        let mut subs: Vec<&str> = self.submodels.iter().map(|s| s.id.as_str()).collect();
        subs.sort_unstable();
        if subs.windows(2).any(|w| w[0] == w[1]) {
            return Err(AasError::Invalid("duplicate submodel id".to_string()));
        }
        Ok(())
    }
}
