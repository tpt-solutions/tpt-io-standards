//! HL7 v2.x messages: reader, writer and typed access to fields and components.
//!
//! An HL7 v2 message is a sequence of segments, each a line of fields separated by `|`. The first
//! segment is `MSH`, whose fourth character sets the field separator and whose second field holds
//! the encoding characters (component `^`, repetition `~`, escape `\`, subcomponent `&`). Fields
//! may repeat (`~`) and contain components (`^`). This crate keeps every field's raw text, so a
//! message round-trips exactly, and gives typed access to the separators and common fields.
//!
//! ```
//! let msg = b"MSH|^~\\&|LAB|HOSP|||202401011200||ADT^A01|MSG001|P|2.5\r\
//! PID|1||12345^^^HOSP^MR||Doe^John^A\r";
//! let m = tpt_io_hl7v2::parse(msg).unwrap();
//! assert_eq!(m.message_type().as_deref(), Some("ADT^A01"));
//! assert_eq!(m.segment("PID").unwrap().field(5).unwrap(), "Doe^John^A");
//! ```
//!
//! Read and write are text-level: the reader does not validate segment structure against the HL7
//! version's tables, and it does not decode escape sequences.
//!
//! Spec access: HL7 v2 is published by Health Level Seven International. Its licence terms apply to
//! the standard; no text is copied into this crate.

#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]

#[cfg(any(feature = "alloc", feature = "std"))]
#[allow(unused_imports)]
#[macro_use]
extern crate alloc;

pub mod error;
pub mod message;

pub use error::Hl7Error;
pub use message::{parse, write, Message, Segment, Separators};
