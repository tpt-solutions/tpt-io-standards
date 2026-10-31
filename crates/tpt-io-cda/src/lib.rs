//! HL7 Clinical Document Architecture (CDA R2 and C-CDA) documents: a read-only reader.
//!
//! A CDA document is an XML `ClinicalDocument`. It identifies the document (`id`, `code`, `title`,
//! `effectiveTime`), the patient (`recordTarget/patientRole/patient/name`), and carries a structured
//! body of sections, each with a title and a code. This crate reads that identity and the section
//! inventory into a [`ClinicalDocument`].
//!
//! ```
//! let xml = br#"<ClinicalDocument xmlns="urn:hl7-org:v3">
//!   <id root="2.16.840.1.113883" extension="DOC-1"/>
//!   <title>Discharge Summary</title>
//!   <effectiveTime value="20240101120000"/>
//!   <recordTarget><patientRole><patient><name><given>Ada</given><family>Lovelace</family></name></patient></patientRole></recordTarget>
//!   <component><structuredBody>
//!     <component><section><title>Medications</title><code code="10160-0"/></section></component>
//!   </structuredBody></component>
//! </ClinicalDocument>"#;
//! let doc = tpt_io_cda::parse(xml).unwrap();
//! assert_eq!(doc.patient.as_deref(), Some("Ada Lovelace"));
//! assert_eq!(doc.sections[0].title.as_deref(), Some("Medications"));
//! ```
//!
//! Section narrative text, entries, and the template ids are not decoded.
//!
//! Spec access: CDA is an HL7 standard and C-CDA is published by HL7 under its licence terms. No text is
//! copied into this crate.

#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]

#[cfg(any(feature = "alloc", feature = "std"))]
#[allow(unused_imports)]
#[macro_use]
extern crate alloc;

pub mod error;
pub mod document;

pub use document::{parse, ClinicalDocument, Section};
pub use error::CdaError;
