//! DICOM parser for the `tpt-io` workspace.
//!
//! Implements DICOM PS3.5 (Data Structures and Encoding), PS3.6 (Data Dictionary),
//! PS3.7 (Message Exchange), and PS3.10 (Media Storage). Supports both explicit
//! and implicit VR transfer syntaxes, pixel data extraction, tag dictionary
//! lookups, UID registry, study/series/instance hierarchy navigation, and
//! anonymization utilities.
//!
//! ```
//! use tpt_io_core::Reader;
//! use tpt_io_dicom::parser::DicomReader;
//!
//! // Build a minimal DICOM file in memory.
//! let mut dcm = vec![0u8; 128]; // preamble
//! dcm.extend_from_slice(b"DICM");
//! // File meta: Transfer Syntax UID (0002,0010) — explicit VR LE.
//! dcm.extend_from_slice(&0x0002u16.to_le_bytes());
//! dcm.extend_from_slice(&0x0010u16.to_le_bytes());
//! dcm.extend_from_slice(b"UI");
//! let ts = b"1.2.840.10008.1.2.1\0";
//! dcm.extend_from_slice(&(ts.len() as u16).to_le_bytes());
//! dcm.extend_from_slice(ts);
//! // Dataset: SOP Class UID (0008,0016).
//! dcm.extend_from_slice(&0x0008u16.to_le_bytes());
//! dcm.extend_from_slice(&0x0016u16.to_le_bytes());
//! dcm.extend_from_slice(b"UI");
//! let sop = b"1.2.840.10008.5.1.4.1.1.2\0";
//! dcm.extend_from_slice(&(sop.len() as u16).to_le_bytes());
//! dcm.extend_from_slice(sop);
//!
//! let file = DicomReader::new().parse(&dcm).unwrap();
//! assert_eq!(file.dataset.string(tpt_io_dicom::model::Tag::new(0x0008, 0x0016)),
//!            Some("1.2.840.10008.5.1.4.1.1.2"));
//! ```
//!
//! Downstream `sci-image` integration is behind a feature flag as a stub until
//! that crate publishes (see [`integration`]).

#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]

#[cfg(any(feature = "alloc", feature = "std"))]
extern crate alloc;

pub mod anonymize;
pub mod error;
pub mod hierarchy;
pub mod integration;
pub mod model;
pub mod parser;
pub mod pixel;
pub mod streaming;
pub mod tag;
pub mod uid;
pub mod writer;

pub use error::DicomError;
pub use model::{Dataset, DicomFile, Element, Tag, Value, VR};
pub use parser::{parse_dicom, DicomReader};
pub use tag::{lookup_tag, TagInfo, VRCode};
pub use uid::{SopClass, TransferSyntax};
pub use writer::DicomWriter;

#[cfg(any(feature = "alloc", feature = "std"))]
pub use tpt_io_core::ParseError;
pub use tpt_io_core::{
    Encoding, FormatExtension, NoProgress, Progress, ProgressFn, Reader, Schema,
    ValidationSettings, Validator, Writer,
};
