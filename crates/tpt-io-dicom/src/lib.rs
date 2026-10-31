//! DICOM parser for PS3.5 / PS3.6 / PS3.7 / PS3.10.
//!
//! Supports the three native transfer syntaxes (Implicit VR Little Endian,
//! Explicit VR Little Endian, Explicit VR Big Endian), sequences of items
//! with defined and undefined lengths, a curated standard tag dictionary
//! and UID registry, PS3.10 file reading/writing, raw pixel-data extraction,
//! RLE Lossless decompression with streaming frame access, study/series/
//! instance navigation, and anonymization utilities.
//!
//! # Example
//!
//! ```no_run
//! use tpt_io_dicom::file::DicomFile;
//!
//! let bytes = std::fs::read("image.dcm").unwrap();
//! let dicom = DicomFile::parse(&bytes).unwrap();
//! println!("patient: {:?}", dicom.dataset.string(tpt_io_dicom::tag::Tag::new(0x0010, 0x0010)));
//! ```

#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]

#[cfg(any(feature = "alloc", feature = "std"))]
extern crate alloc;

pub mod anonymize;
pub mod dataset;
pub mod dict;
pub mod element;
pub mod error;
pub mod file;
pub mod hierarchy;
pub mod pixel;
#[cfg(feature = "sci-image-integration")]
pub mod sci_image;
pub mod tag;
pub mod uid;
pub mod vr;
pub mod wsi;

pub use dataset::{Dataset, Encoding};
pub use element::{DataElement, Item, Value};
pub use error::{DicomError, Result};
pub use file::DicomFile;
pub use tag::Tag;
pub use tpt_io_core::{
    NoProgress, Progress, ProgressFn, Reader as CoreReader, Schema, ValidationSettings, Validator,
    Writer as CoreWriter,
};
pub use uid::{SopClass, TransferSyntax};
pub use vr::Vr;
