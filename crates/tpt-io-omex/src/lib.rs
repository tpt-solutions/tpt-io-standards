//! COMBINE archives (OMEX): read and write the ZIP container that bundles a model with its simulation
//! description, and the SED-ML summary reader for the simulation part.
//!
//! An OMEX archive holds a `manifest.xml` listing each content file with its location and format URI, and
//! the files themselves. [`parse`] checks that every listed file is present and returns the contents;
//! [`write`] builds a new archive with stored members. [`sedml::parse_sedml`] lists the models, simulations,
//! tasks and outputs of a SED-ML file found in an archive.
//!
//! ```
//! use tpt_io_omex::{Archive, Content};
//!
//! let archive = Archive {
//!     contents: vec![Content {
//!         location: "./model.xml".to_string(),
//!         format: "http://identifiers.org/combine.specifications/sbml".to_string(),
//!         master: true,
//!         data: b"<sbml/>".to_vec(),
//!     }],
//! };
//! let bytes = tpt_io_omex::write(&archive);
//! assert_eq!(tpt_io_omex::parse(&bytes).unwrap(), archive);
//! ```
//!
//! Spec access: the COMBINE archive and SED-ML specifications are open standards. No specification text is
//! copied into this crate. Metadata (`metadata.rdf`) is not interpreted; it is kept as an ordinary content file
//! when present.

#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]

#[cfg(not(any(feature = "alloc", feature = "std")))]
compile_error!(
    "tpt-io-omex requires the `alloc` or `std` feature: its types own text and vectors."
);

#[cfg(any(feature = "alloc", feature = "std"))]
#[allow(unused_imports)]
#[macro_use]
extern crate alloc;

pub mod sedml;

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::fmt::Write as _;

use tpt_io_core::{xml, zip, ParseError};

/// The manifest part name inside an archive.
pub const MANIFEST: &str = "manifest.xml";

const MANIFEST_NS: &str = "http://identifiers.org/combine.specifications/omex-manifest";

/// Errors produced while reading or writing an archive or SED-ML file.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum OmexError {
    /// A malformed archive or XML, with its byte offset.
    Core(ParseError),
    /// The archive or document breaks a rule (missing manifest, listed file not present, wrong root).
    Invalid(String),
}

impl From<ParseError> for OmexError {
    fn from(e: ParseError) -> Self {
        OmexError::Core(e)
    }
}

impl core::fmt::Display for OmexError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            OmexError::Core(e) => write!(f, "{e}"),
            OmexError::Invalid(what) => write!(f, "invalid COMBINE archive: {what}"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for OmexError {}

/// A COMBINE archive: the content files named by its manifest.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Archive {
    /// The contents, in manifest order.
    pub contents: Vec<Content>,
}

impl Archive {
    /// The master file: the content marked `master="true"`, which a reader should open first.
    pub fn master(&self) -> Option<&Content> {
        self.contents.iter().find(|c| c.master)
    }

    /// Find a content file by its manifest location, such as `./model.xml`.
    pub fn content(&self, location: &str) -> Option<&Content> {
        self.contents.iter().find(|c| c.location == location)
    }
}

/// One content file: where it sits, what it is, and its bytes.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Content {
    /// The manifest location, relative to the archive root, such as `./model.xml`.
    pub location: String,
    /// The format URI from the manifest, such as `http://identifiers.org/combine.specifications/sbml`.
    pub format: String,
    /// Whether the manifest marks this file as the master file.
    pub master: bool,
    /// The file contents.
    pub data: Vec<u8>,
}

/// The archive path for a manifest location: the location with a leading `./` or `/` removed.
fn path_of(location: &str) -> &str {
    location.trim_start_matches("./").trim_start_matches('/')
}

fn attr<'a>(el: &'a xml::Element, name: &str) -> Result<&'a str, OmexError> {
    el.attr(name)
        .ok_or_else(|| OmexError::Invalid(alloc::format!("<{}> has no {name} attribute", el.name)))
}

/// Read a COMBINE archive: every manifest entry must name a file that is present in the archive.
pub fn parse(input: &[u8]) -> Result<Archive, OmexError> {
    let manifest = zip::entry(input, MANIFEST)?
        .ok_or_else(|| OmexError::Invalid("archive has no manifest.xml".to_string()))?;
    let root = xml::parse(&manifest)?;
    if root.local_name() != "omexManifest" {
        return Err(OmexError::Invalid("manifest root is not <omexManifest>".to_string()));
    }
    let mut archive = Archive::default();
    for el in root.children_named("content") {
        let location = attr(el, "location")?;
        let path = path_of(location);
        let data = zip::entry(input, path)?.ok_or_else(|| {
            OmexError::Invalid(alloc::format!("manifest lists missing file {path}"))
        })?;
        archive.contents.push(Content {
            location: location.to_string(),
            format: attr(el, "format")?.to_string(),
            master: el.attr("master") == Some("true"),
            data,
        });
    }
    Ok(archive)
}

/// Write a COMBINE archive: a `manifest.xml` listing every content file, and the files, in stored ZIP members.
pub fn write(archive: &Archive) -> Vec<u8> {
    let mut manifest = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    let _ = writeln!(manifest, "<omexManifest xmlns=\"{MANIFEST_NS}\">");
    for c in &archive.contents {
        let master = if c.master { " master=\"true\"" } else { "" };
        let _ = writeln!(
            manifest,
            "<content location=\"{}\" format=\"{}\"{master}/>",
            xml::escape_attr(&c.location),
            xml::escape_attr(&c.format)
        );
    }
    manifest.push_str("</omexManifest>\n");
    let mut files: Vec<(String, Vec<u8>)> = vec![(MANIFEST.to_string(), manifest.into_bytes())];
    for c in &archive.contents {
        files.push((path_of(&c.location).to_string(), c.data.clone()));
    }
    let refs: Vec<(&str, &[u8])> = files.iter().map(|(n, d)| (n.as_str(), d.as_slice())).collect();
    zip::write_stored(&refs)
}
