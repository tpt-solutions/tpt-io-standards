//! IFC (Industry Foundation Classes) 4.0 / 4.3 parser for the `tpt-io`
//! workspace.
//!
//! The crate implements a from-scratch, pure-Rust parser for the
//! STEP-Physical File (SPF / ISO 10303-21) encoding of IFC building data.
//! It tokenizes and parses the file into a structured [`IfcFile`], resolves
//! entity instances and their references, and provides higher-level
//! traversals: relationship graph construction, property set traversal,
//! geometry extraction (B-Rep, CSG, tessellated), schema version detection,
//! spatial structure navigation, and round-trip serialization.
//!
//! ```
//! use tpt_io_core::Reader;
//! use tpt_io_ifc::parser::IfcReader;
//!
//! let src = b"ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION(('ViewDefinition'),'2;1');\nFILE_NAME('t.ifc','2024',(),(),'','','');\nFILE_SCHEMA(('IFC4'));\nENDSEC;\nDATA;\n#1=IFCWALL('g',$,'Wall',$,$,$,$,$);\nENDSEC;\nEND-ISO-10303-21;\n";
//! let file = IfcReader::new().parse(src).unwrap();
//! assert_eq!(file.instance_count(), 1);
//! assert_eq!(file.schema(), Some("IFC4"));
//! ```
//!
//! Parallel entity-instance parsing via Rayon is available behind the
//! `parallel` feature (see [`parser::IfcReader`]). Downstream `eng-*`
//! integrations are behind feature flags as stubs until those crates publish.

#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]

#[cfg(any(feature = "alloc", feature = "std"))]
#[allow(unused_imports)]
#[macro_use]
extern crate alloc;

pub mod error;
pub mod geometry;
pub mod graph;
pub mod model;
pub mod parser;
pub mod property;
pub mod schema;
pub mod spatial;
pub mod token;
pub mod writer;

#[cfg(any(feature = "eng-geometry-integration", feature = "eng-cad-integration"))]
pub mod integration;

pub use error::IfcError;
pub use graph::RelGraph;
pub use model::{EntityInstance, FileDescription, FileName, IfcFile, IfcHeader, IfcValue};
pub use parser::{parse_str, IfcReader, IfcValidator};
pub use property::{Property, PropertySet, PropertyTraversal, PropertyValue};
pub use schema::{detect_schema, IfcSchemaVersion, SchemaMigration};
pub use spatial::{name_of, SpatialNavigator, SpatialNode};
pub use writer::serialize;

#[cfg(any(feature = "alloc", feature = "std"))]
pub use tpt_io_core::ParseError;
pub use tpt_io_core::{
    Encoding, FormatExtension, NoProgress, Progress, ProgressFn, Reader, Schema,
    ValidationSettings, Validator, Writer,
};
