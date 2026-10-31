//! FITS files: header cards, header/data units (HDUs) and image pixels.
//!
//! A FITS file is a sequence of HDUs. Each HDU has a header of 80-character cards in 2880-byte
//! blocks, ending with an `END` card, followed by a data array padded to a block boundary. This
//! crate reads every HDU, parses its cards into typed [`Value`]s, and decodes image pixels with
//! `BZERO`/`BSCALE` applied. It writes the same structure back.
//!
//! ```
//! let mut hdu = tpt_io_fits::Hdu::image(8, &[2, 2]);
//! hdu.data = vec![1, 2, 3, 4];
//! let file = tpt_io_fits::Fits { hdus: vec![hdu] };
//! let bytes = tpt_io_fits::write(&file);
//! let back = tpt_io_fits::parse(&bytes).unwrap();
//! assert_eq!(back.hdus[0].pixels().unwrap(), vec![1.0, 2.0, 3.0, 4.0]);
//! ```
//!
//! Supported: image HDUs with `BITPIX` 8, 16, 32, 64, -32 and -64; the primary HDU and image
//! extensions. Binary and ASCII tables are read as raw bytes with their header cards and are not
//! decoded into columns.
//!
//! Spec access: FITS is defined by the FITS Standard and FITS Support Office documents, which
//! are published openly. No text is copied into this crate.

#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]

#[cfg(any(feature = "alloc", feature = "std"))]
#[allow(unused_imports)]
#[macro_use]
extern crate alloc;

pub mod card;
pub mod error;
pub mod hdu;

pub use card::{Card, Value};
pub use error::FitsError;
pub use hdu::{parse, write, Fits, Hdu};
