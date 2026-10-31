//! Error type for the URDF/SDF reader, built on [`tpt_io_core::ParseError`].

#[cfg(not(any(feature = "alloc", feature = "std")))]
compile_error!("tpt-io-urdf requires the `alloc` or `std` feature: its error type owns text.");

use alloc::string::String;

use tpt_io_core::ParseError;

/// Errors produced while reading, writing or validating a robot description.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum UrdfError {
    /// Malformed XML or a missing or malformed attribute, with its byte offset.
    Core(ParseError),
    /// The document parsed but is not a valid robot description.
    Invalid(String),
}

impl From<ParseError> for UrdfError {
    fn from(e: ParseError) -> Self {
        UrdfError::Core(e)
    }
}

impl core::fmt::Display for UrdfError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            UrdfError::Core(e) => write!(f, "{e}"),
            UrdfError::Invalid(what) => write!(f, "invalid robot description: {what}"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for UrdfError {}
