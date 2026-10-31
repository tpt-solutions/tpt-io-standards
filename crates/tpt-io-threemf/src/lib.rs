//! 3MF (3D Manufacturing Format) packages: read and write the model part of a 3MF archive.
//!
//! A 3MF file is a ZIP container whose `3D/3dmodel.model` part is XML. [`parse`] reads that part into a
//! [`Model`]: a list of objects (each a triangle mesh with an id) and a list of build items (which objects
//! are placed on the build plate, with an optional transform). [`write`] produces a minimal package with
//! stored (uncompressed) members, which 3MF readers accept.
//!
//! ```
//! use tpt_io_threemf::{Item, Model, Object};
//!
//! let model = Model {
//!     unit: "millimeter".to_string(),
//!     objects: vec![Object {
//!         id: 1,
//!         name: None,
//!         vertices: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
//!         triangles: vec![[0, 1, 2]],
//!     }],
//!     build: vec![Item { object_id: 1, transform: None }],
//! };
//! let bytes = tpt_io_threemf::write(&model);
//! assert_eq!(tpt_io_threemf::parse(&bytes).unwrap(), model);
//! ```
//!
//! Spec access: the 3MF Core Specification is published by the 3MF Consortium under a royalty-free licence.
//! No specification text is copied into this crate. Materials, colours, slices, beam lattices and production
//! extensions are not read or written.

#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]

#[cfg(not(any(feature = "alloc", feature = "std")))]
compile_error!("tpt-io-threemf requires the `alloc` or `std` feature: its types own text and vectors.");

#[cfg(any(feature = "alloc", feature = "std"))]
#[allow(unused_imports)]
#[macro_use]
extern crate alloc;

mod reader;
mod writer;

use alloc::string::String;
use alloc::vec::Vec;

use tpt_io_core::ParseError;

/// The path of the XML model part inside a 3MF package.
pub const MODEL_PART: &str = "3D/3dmodel.model";

/// Errors produced while reading or writing a 3MF package.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ThreeMfError {
    /// A malformed archive, XML or number, with its byte offset.
    Core(ParseError),
    /// The package breaks a rule (missing model part, unknown object, wrong transform length).
    Invalid(String),
}

impl From<ParseError> for ThreeMfError {
    fn from(e: ParseError) -> Self {
        ThreeMfError::Core(e)
    }
}

impl core::fmt::Display for ThreeMfError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            ThreeMfError::Core(e) => write!(f, "{e}"),
            ThreeMfError::Invalid(what) => write!(f, "invalid 3MF package: {what}"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for ThreeMfError {}

/// The model described by a 3MF package.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Model {
    /// The length unit named by the model, `millimeter` unless the file says otherwise.
    pub unit: String,
    /// Every object in `resources`, in file order.
    pub objects: Vec<Object>,
    /// The build items, in file order.
    pub build: Vec<Item>,
}

impl Model {
    /// Find an object by its id.
    pub fn object(&self, id: u32) -> Option<&Object> {
        self.objects.iter().find(|o| o.id == id)
    }
}

/// One object: a triangle mesh with an id that build items refer to.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Object {
    /// The object id, unique within the model.
    pub id: u32,
    /// The optional `name` attribute.
    pub name: Option<String>,
    /// Vertex positions.
    pub vertices: Vec<[f32; 3]>,
    /// Triangles as three indices into `vertices`.
    pub triangles: Vec<[u32; 3]>,
}

/// One build item: an object placed on the build plate.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Item {
    /// The id of the object this item places.
    pub object_id: u32,
    /// The optional transform: twelve values, row-major 3×4 (rotation and scale, then translation).
    pub transform: Option<[f32; 12]>,
}

/// Read a 3MF package (a ZIP archive) and return its model.
pub fn parse(input: &[u8]) -> Result<Model, ThreeMfError> {
    reader::parse_package(input)
}

/// Read only the XML model part, as found inside a package at [`MODEL_PART`].
pub fn parse_model_xml(input: &[u8]) -> Result<Model, ThreeMfError> {
    reader::parse_model(input)
}

/// Write a model as a 3MF package.
pub fn write(model: &Model) -> Vec<u8> {
    writer::package(model)
}

/// Serialise the XML model part for a model, without the surrounding package.
pub fn model_xml(model: &Model) -> String {
    writer::model_xml(model)
}
