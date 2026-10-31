//! MCAP (`.mcap`) robotics log files: reader and writer for records, schemas, channels,
//! messages and uncompressed chunks.
//!
//! An MCAP file is a magic string, a header, and a sequence of length-prefixed records.
//! Messages are grouped into channels, and each channel points at a schema. Records may be
//! stored directly in the data section or inside chunks. This crate reads both, skipping the
//! index and summary sections that a reader does not need to recover the messages.
//!
//! ```
//! let mcap = tpt_io_mcap::Mcap {
//!     profile: "ros2".into(),
//!     library: "example".into(),
//!     schemas: vec![],
//!     channels: vec![tpt_io_mcap::Channel {
//!         id: 0, schema_id: 0, topic: "/chatter".into(),
//!         message_encoding: "cdr".into(), metadata: vec![],
//!     }],
//!     messages: vec![tpt_io_mcap::Message {
//!         channel_id: 0, sequence: 1, log_time: 10, publish_time: 10, data: vec![1, 2, 3],
//!     }],
//! };
//! let bytes = tpt_io_mcap::write(&mcap);
//! assert_eq!(tpt_io_mcap::parse(&bytes).unwrap(), mcap);
//! ```
//!
//! Compressed chunks (`lz4`, `zstd`) return [`McapError::Unsupported`]. Chunk CRCs and the
//! data-section CRC are not verified.
//!
//! Spec access: the MCAP file format is published by Foxglove under an open licence
//! (Apache-2.0). This crate follows the published record layout; no spec text is copied.

#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]

#[cfg(any(feature = "alloc", feature = "std"))]
#[allow(unused_imports)]
#[macro_use]
extern crate alloc;

pub mod error;
pub mod format;

pub use error::McapError;
pub use format::{parse, write, Channel, Mcap, Message, Schema};
