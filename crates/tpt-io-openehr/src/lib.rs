//! openEHR compositions in the Reference Model JSON form: reader (read only).
//!
//! An openEHR composition is the unit of clinical documentation in an electronic health record. Its
//! Reference Model JSON carries an `_type` of `COMPOSITION`, a `name`, the `archetype_details` (archetype
//! and template ids and the RM version), the `language`, and a `content` list of entries (observations,
//! evaluations, actions and so on), each with its own `_type`, name and archetype node id. This crate reads
//! that identity and the top-level entries.
//!
//! ```
//! let json = br#"{"_type":"COMPOSITION",
//!   "name":{"_type":"DV_TEXT","value":"Vital signs"},
//!   "archetype_details":{"archetype_id":{"value":"openEHR-EHR-COMPOSITION.encounter.v1"},"rm_version":"1.0.4"},
//!   "language":{"code_string":"en"},
//!   "content":[{"_type":"OBSERVATION","archetype_node_id":"openEHR-EHR-OBSERVATION.blood_pressure.v2",
//!               "name":{"value":"Blood pressure"}}]}"#;
//! let c = tpt_io_openehr::parse(json).unwrap();
//! assert_eq!(c.entries[0].kind, "OBSERVATION");
//! ```
//!
//! Read only. Archetypes (ADL) are not parsed, and the entry trees below the top level are not walked.
//!
//! Spec access: openEHR is published by the openEHR Foundation under its open licence. No specification
//! text is copied into this crate.

#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]

#[cfg(any(feature = "alloc", feature = "std"))]
#[allow(unused_imports)]
#[macro_use]
extern crate alloc;

pub mod composition;
pub mod error;

pub use composition::{parse, Composition, Entry};
pub use error::OpenEhrError;
