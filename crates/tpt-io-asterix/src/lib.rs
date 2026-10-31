//! ASTERIX surveillance data (Eurocontrol): data block framing, FSPEC decoding and a CAT048 subset.
//!
//! ASTERIX messages are binary data blocks: a category octet, a two-octet length (big-endian, counting
//! the header), and records. Each record starts with a Field Specification (FSPEC): a run of octets whose
//! bits, read from the top, say which data items the record carries. Item order follows the category's
//! User Application Profile (UAP), so the FSPEC bit position names the item.
//!
//! This crate frames data blocks and decodes FSPEC bits for every category. The item table is supplied
//! for category 048 (Monoradar Target Reports) for the fixed-length items it covers: data source
//! identifier, time of day, measured position, Mode-3/A code, flight level, aircraft address and track
//! number. A record that uses any other item is reported as [`AsterixError::UnsupportedItem`], because
//! the length of an unknown item cannot be guessed safely.
//!
//! ```
//! // One CAT048 block with one record: I048/010 (SAC 1, SIC 2) and I048/140 (time 0x000080 = 1.0 s).
//! let block = [0x30, 0x00, 0x09, 0xC0, 0x01, 0x02, 0x00, 0x00, 0x80];
//! let blocks = tpt_io_asterix::parse(&block).unwrap();
//! assert_eq!(blocks[0].records[0].time_of_day_s(), Some(1.0));
//! ```
//!
//! Spec access: ASTERIX categories are published by EUROCONTROL. The category 048 field layout used
//! here follows the published item descriptions. No specification text is copied into this crate.

#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]

#[cfg(any(feature = "alloc", feature = "std"))]
#[allow(unused_imports)]
#[macro_use]
extern crate alloc;

pub mod error;
pub mod frame;

pub use error::AsterixError;
pub use frame::{parse, Block, Cat048Record, Record};
