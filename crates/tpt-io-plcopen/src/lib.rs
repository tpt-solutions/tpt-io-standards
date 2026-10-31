//! PLCopen XML (TC6 exchange format for IEC 61131-3 programs): reader.
//!
//! A PLCopen project lists its program organisation units (POUs) under `types/pous`. Each POU is a
//! program, function block or function with an interface of variables and a body in one of the five
//! IEC languages. This crate reads the project name, the POUs, their variables and, for the textual
//! languages (Structured Text and Instruction List), the body text.
//!
//! ```
//! let xml = br#"<project><fileHeader productName="demo"/><types><pous>
//!   <pou name="Counter" pouType="program"><interface><localVars>
//!     <variable name="n"><type><INT/></type></variable>
//!   </localVars></interface>
//!   <body><ST><xhtml>n := n + 1;</xhtml></ST></body></pou>
//! </pous></types></project>"#;
//! let p = tpt_io_plcopen::parse(xml).unwrap();
//! assert_eq!(p.pous[0].variables[0].type_name, "INT");
//! assert_eq!(p.pous[0].body.as_ref().unwrap().text, "n := n + 1;");
//! ```
//!
//! Read only. Graphical bodies (FBD, LD, SFC) are recognised by language but their networks are not
//! decoded. Global variables, configurations and resources are not read.
//!
//! Spec access: the PLCopen XML schema (TC6) is published by PLCopen under its own terms. No
//! schema text is copied into this crate.

#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]

#[cfg(any(feature = "alloc", feature = "std"))]
#[allow(unused_imports)]
#[macro_use]
extern crate alloc;

pub mod error;
pub mod model;
pub mod reader;

pub use error::PlcOpenError;
pub use model::{Body, Language, Pou, PouKind, Project, Variable};
pub use reader::parse;
