//! BIM Collaboration Format (BCF 2.1) projects: topics, comments and viewpoint references.
//!
//! A BCF project is a ZIP archive. `bcf.version` names the version, and each topic lives in a folder named
//! by its GUID with a `markup.bcfxml` part holding the title, status, comments and viewpoint references.
//! This crate reads and writes those markup parts. Viewpoint files (`*.bcfv`), snapshots and linked models
//! are not parsed: a viewpoint reference keeps only its GUID and file name.
//!
//! ```
//! use tpt_io_bcf::{Project, Topic};
//!
//! let project = Project {
//!     version: "2.1".to_string(),
//!     topics: vec![Topic {
//!         guid: "2d6a5a1e-0000-4000-8000-000000000001".to_string(),
//!         title: "Wall clash".to_string(),
//!         ..Topic::default()
//!     }],
//! };
//! let bytes = tpt_io_bcf::write(&project);
//! let back = tpt_io_bcf::parse(&bytes).unwrap();
//! assert_eq!(back.topics[0].title, "Wall clash");
//! ```
//!
//! Spec access: BCF is an open buildingSMART standard. No specification text is copied into this crate.

#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]

#[cfg(not(any(feature = "alloc", feature = "std")))]
compile_error!("tpt-io-bcf requires the `alloc` or `std` feature: its types own text and vectors.");

#[cfg(any(feature = "alloc", feature = "std"))]
#[allow(unused_imports)]
#[macro_use]
extern crate alloc;

mod markup;

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::fmt::Write as _;

use tpt_io_core::{xml, zip, ParseError};

/// The version written to `bcf.version` and assumed when that part is absent.
pub const DEFAULT_VERSION: &str = "2.1";

/// Errors produced while reading or writing a BCF project.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum BcfError {
    /// A malformed archive or XML, with its byte offset.
    Core(ParseError),
    /// The project breaks a rule (a topic without a GUID, a markup part without a topic).
    Invalid(String),
}

impl From<ParseError> for BcfError {
    fn from(e: ParseError) -> Self {
        BcfError::Core(e)
    }
}

impl core::fmt::Display for BcfError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            BcfError::Core(e) => write!(f, "{e}"),
            BcfError::Invalid(what) => write!(f, "invalid BCF project: {what}"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for BcfError {}

/// A BCF project: its version and topics.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Project {
    /// The version from `bcf.version`, such as `2.1`.
    pub version: String,
    /// The topics, in archive order.
    pub topics: Vec<Topic>,
}

/// One issue, clash, request or note, with its comments and viewpoint references.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Topic {
    /// The topic GUID, which also names its folder in the archive.
    pub guid: String,
    /// The title.
    pub title: String,
    /// The optional `TopicType` attribute, such as `Issue` or `Clash`.
    pub topic_type: Option<String>,
    /// The optional `TopicStatus` attribute, such as `Open` or `Closed`.
    pub status: Option<String>,
    /// The optional priority.
    pub priority: Option<String>,
    /// The optional creation author.
    pub creation_author: Option<String>,
    /// The optional creation date, kept as written (ISO 8601).
    pub creation_date: Option<String>,
    /// The comments, in file order.
    pub comments: Vec<Comment>,
    /// The viewpoint references, in file order.
    pub viewpoints: Vec<ViewpointRef>,
}

/// One comment on a topic.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Comment {
    /// The comment GUID.
    pub guid: String,
    /// The optional author.
    pub author: Option<String>,
    /// The optional date, kept as written.
    pub date: Option<String>,
    /// The comment text.
    pub text: String,
}

/// A reference from a topic to a viewpoint.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ViewpointRef {
    /// The viewpoint GUID.
    pub guid: String,
    /// The viewpoint file name, such as `viewpoint.bcfv`, when given.
    pub file: Option<String>,
}

/// Read a BCF project (a ZIP archive) and return its version and topics.
pub fn parse(input: &[u8]) -> Result<Project, BcfError> {
    let version = match zip::entry(input, "bcf.version")? {
        Some(bytes) => {
            let root = xml::parse(&bytes)?;
            root.attr("VersionId").unwrap_or(DEFAULT_VERSION).to_string()
        }
        None => DEFAULT_VERSION.to_string(),
    };
    let mut topics = Vec::new();
    for entry in zip::entries(input)? {
        if entry.name.ends_with("markup.bcfxml") {
            let root = xml::parse(&entry.data)?;
            topics.push(markup::topic(&root)?);
        }
    }
    Ok(Project { version, topics })
}

/// Write a BCF project: `bcf.version` plus one `markup.bcfxml` part per topic, in stored ZIP members.
pub fn write(project: &Project) -> Vec<u8> {
    let mut version = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    let _ = writeln!(
        version,
        "<Version VersionId=\"{}\"><DetailedVersion>{}</DetailedVersion></Version>",
        xml::escape_attr(&project.version),
        xml::escape_text(&project.version)
    );
    let mut files: Vec<(String, Vec<u8>)> = vec![("bcf.version".to_string(), version.into_bytes())];
    for topic in &project.topics {
        files.push((
            alloc::format!("{}/markup.bcfxml", topic.guid),
            markup::markup(topic).into_bytes(),
        ));
    }
    let refs: Vec<(&str, &[u8])> = files.iter().map(|(n, d)| (n.as_str(), d.as_slice())).collect();
    zip::write_stored(&refs)
}
