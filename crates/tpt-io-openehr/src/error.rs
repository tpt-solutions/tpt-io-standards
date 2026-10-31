//! Error type for the openEHR reader, built on [`tpt_io_core::ParseError`].

#[cfg(not(any(feature = "alloc", feature = "std")))]
compile_error!("tpt-io-openehr requires the `alloc` or `std` feature: its error type owns text.");

use alloc::string::String;

use tpt_io_core::ParseError;

/// Errors produced while reading an openEHR composition.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum OpenEhrError {
    /// Malformed JSON, with its byte offset.
    Core(ParseError),
    /// The JSON is not a composition (wrong `_type`, or not an object).
    Invalid(String),
}

impl From<ParseError> for OpenEhrError {
    fn from(e: ParseError) -> Self {
        OpenEhrError::Core(e)
    }
}

impl core::fmt::Display for OpenEhrError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            OpenEhrError::Core(e) => write!(f, "{e}"),
            OpenEhrError::Invalid(what) => write!(f, "invalid openEHR composition: {what}"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for OpenEhrError {}
