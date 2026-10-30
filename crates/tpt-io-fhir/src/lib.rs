//! HL7 FHIR R4/R5 deserializer for JSON and XML representations.
//!
//! `tpt-io-fhir` parses FHIR resources from both JSON and XML wire formats
//! into strongly-typed Rust models, validates them against profiles and NZ HIPC
//! 2020 compliance rules, resolves cross-resource references, evaluates FHIRPath
//! expressions, and supports batch import/export via FHIR bundles.
//!
//! ## Feature flags
//!
//! | Feature | Effect |
//! |---|---|
//! | `json` | JSON deserialization via `serde_json` (implies `alloc`). |
//! | `xml` | XML deserialization via `quick-xml` (implies `std`). |
//! | `validation` | Profile + NZ HIPC 2020 compliance validation (implies `alloc`). |
//! | `reference` | Cross-resource reference resolution (implies `alloc`). |
//! | `bundle` | Bundle traversal + pagination helpers (implies `alloc`). |
//! | `fhirpath` | FHIRPath expression evaluator (implies `alloc`). |
//! | `batch` | Batch import/export utilities (implies `alloc`). |
//! | `medical-patient-integration` | Stub conversion to a future `tpt-medical-patient` crate. |
//!
//! The `std` feature (default) enables JSON deserialization. Build with
//! `--all-features` for the full feature set.

#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]

#[cfg(any(feature = "alloc", feature = "std"))]
#[allow(unused_imports)]
#[macro_use]
extern crate alloc;

pub mod error;
pub mod model;

#[cfg(feature = "batch")]
pub mod batch;
#[cfg(feature = "bundle")]
pub mod bundle;
#[cfg(feature = "fhirpath")]
pub mod fhirpath;
#[cfg(feature = "medical-patient-integration")]
pub mod integration;
#[cfg(feature = "json")]
pub mod json;
#[cfg(feature = "reference")]
pub mod reference;
#[cfg(feature = "validation")]
pub mod validation;
#[cfg(feature = "xml")]
pub mod xml;

pub use error::FhirError;
pub use model::{
    Bundle, BundleEntry, BundleLink, BundleRequest, BundleResponse, BundleSearch, BundleType,
    CodeableConcept, Coding, Contact, ContactPoint, Extension, HumanName, Identifier, Meta,
    Narrative, Observation, ObservationComponent, Patient, PatientCommunication, Period, Quantity,
    Range, Ratio, Reference, Resource, ResourceKind, Signature,
};

#[cfg(feature = "bundle")]
pub use bundle::BundleCursor;

#[cfg(any(feature = "alloc", feature = "std"))]
pub use tpt_io_core::ParseError;
pub use tpt_io_core::{
    Encoding, FormatExtension, NoProgress, Progress, ProgressFn, Reader, Schema,
    ValidationSettings, Validator, Writer,
};

/// The FHIR versions this crate supports.
pub const SUPPORTED_VERSIONS: &[&str] = &["R4", "R5"];

/// The FHIR R4 canonical base URL.
pub const FHIR_R4_BASE: &str = "http://hl7.org/fhir/R4";

/// The FHIR R5 canonical base URL.
pub const FHIR_R5_BASE: &str = "http://hl7.org/fhir/R5";
