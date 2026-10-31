//! mzML mass spectrometry files: reader for spectra and their binary arrays (read only).
//!
//! An mzML file lists its spectra under `run/spectrumList`. Each spectrum carries controlled-vocabulary
//! parameters (MS level, scan start time) and `binaryDataArray` elements whose base64 `binary` text
//! holds the m/z and intensity values, as little-endian 32- or 64-bit floats. This crate reads the
//! spectra and decodes uncompressed arrays. Zlib-compressed arrays are reported as unsupported.
//!
//! ```
//! let xml = br#"<mzML><run><spectrumList count="1">
//!   <spectrum index="0" id="scan=1" defaultArrayLength="2">
//!     <cvParam accession="MS:1000511" name="ms level" value="1"/>
//!     <binaryDataArrayList count="2">
//!       <binaryDataArray encodedLength="32">
//!         <cvParam accession="MS:1000523" name="64-bit float"/>
//!         <cvParam accession="MS:1000576" name="no compression"/>
//!         <cvParam accession="MS:1000514" name="m/z array"/>
//!         <binary>AAAAAAAAAAAAAAAAAAAA/wAAAAAAAAAA</binary>
//!       </binaryDataArray>
//!     </binaryDataArrayList>
//!   </spectrum>
//! </spectrumList></run></mzML>"#;
//! let run = tpt_io_mzml::parse(xml).unwrap();
//! assert_eq!(run.spectra[0].ms_level, Some(1));
//! ```
//!
//! Spec access: mzML is defined by the HUPO-PSI and published openly; the controlled vocabulary is
//! published by PSI. No text is copied into this crate.

#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]

#[cfg(any(feature = "alloc", feature = "std"))]
#[allow(unused_imports)]
#[macro_use]
extern crate alloc;

pub mod base64;
pub mod error;
pub mod run;

pub use error::MzmlError;
pub use run::{parse, Run, Spectrum};
