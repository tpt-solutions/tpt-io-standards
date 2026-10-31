//! Error type for the mesh readers, built on [`tpt_io_core::ParseError`].

#[cfg(not(any(feature = "alloc", feature = "std")))]
compile_error!("tpt-io-mesh requires the `alloc` or `std` feature: its error type owns text.");

use alloc::string::String;

use tpt_io_core::ParseError;

/// Errors produced while reading or writing a mesh.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum MeshError {
    /// A malformed number, header, or truncated data, with its byte offset.
    Core(ParseError),
    /// The mesh breaks a rule (a face refers to a missing vertex, a face has fewer than three corners).
    Invalid(String),
}

impl From<ParseError> for MeshError {
    fn from(e: ParseError) -> Self {
        MeshError::Core(e)
    }
}

impl core::fmt::Display for MeshError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            MeshError::Core(e) => write!(f, "{e}"),
            MeshError::Invalid(what) => write!(f, "invalid mesh: {what}"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for MeshError {}
