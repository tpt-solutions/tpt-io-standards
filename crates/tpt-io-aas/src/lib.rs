//! Asset Administration Shell (AAS) JSON environments (IDTA Part 1, V3 serialisation): reader.
//!
//! An AAS environment is a JSON object with three arrays: `assetAdministrationShells`,
//! `submodels` and `conceptDescriptions`. Each shell names its asset and the submodels it refers
//! to; each submodel lists its submodel elements. This crate reads that structure into an
//! [`Environment`] and checks that identifiers are unique.
//!
//! ```
//! let json = br#"{
//!   "assetAdministrationShells": [{"id": "urn:shell:1", "idShort": "Pump",
//!     "assetInformation": {"globalAssetId": "urn:asset:1"},
//!     "submodels": [{"type": "ModelReference", "keys": [{"type": "Submodel", "value": "urn:sm:1"}]}]}],
//!   "submodels": [{"id": "urn:sm:1", "idShort": "Nameplate",
//!     "submodelElements": [{"idShort": "Manufacturer", "modelType": "Property"}]}]
//! }"#;
//! let env = tpt_io_aas::parse(json).unwrap();
//! assert_eq!(env.shells[0].global_asset_id.as_deref(), Some("urn:asset:1"));
//! assert_eq!(env.submodels[0].elements[0].model_type, "Property");
//! env.validate().unwrap();
//! ```
//!
//! Read only. The XML and V2 serialisations are not read, and submodel element values are not
//! decoded.
//!
//! Spec access: the AAS metamodel and JSON serialisation are published by IDTA (Industrial Digital
//! Twin Association) under their open terms. No specification text is copied into this crate.

#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]

#[cfg(any(feature = "alloc", feature = "std"))]
#[allow(unused_imports)]
#[macro_use]
extern crate alloc;

pub mod error;
pub mod model;
pub mod reader;

pub use error::AasError;
pub use model::{Environment, Shell, Submodel, SubmodelElement};
pub use reader::parse;
