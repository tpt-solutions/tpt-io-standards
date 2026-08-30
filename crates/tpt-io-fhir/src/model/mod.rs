//! FHIR resource data models.
//!
//! Strongly-typed structs for the most common resources (Patient, Observation,
//! Bundle) plus shared data types and a generic [`resources::Resource`] enum
//! that can hold any resource kind.

pub mod common;
pub mod resources;

pub use common::{
    Address, Attachment, CodeableConcept, Coding, Contact, ContactPoint, Extension, HumanName,
    Identifier, Meta, Narrative, Period, Quantity, Range, Ratio, Reference, Signature,
};
pub use resources::{
    Bundle, BundleEntry, BundleLink, BundleRequest, BundleResponse, BundleSearch, BundleType,
    Observation, ObservationComponent, Patient, PatientCommunication, Resource, ResourceKind,
};
