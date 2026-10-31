//! Error type for the OME readers, built on [`tpt_io_core::ParseError`].

#[cfg(not(any(feature = "alloc", feature = "std")))]
compile_error!("tpt-io-ome requires the `alloc` or `std` feature: its error type owns text.");

use alloc::string::String;

use tpt_io_core::ParseError;

/// Errors produced while reading OME metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum OmeError {
    /// Malformed XML or JSON, with its byte offset.
    Core(ParseError),
    /// The document parsed but lacks the OME element or field it needs.
    Invalid(String),
    /// A TIFF file that could not be read for its image descriptions.
    Tiff(String),
}

impl From<ParseError> for OmeError {
    fn from(e: ParseError) -> Self {
        OmeError::Core(e)
    }
}

impl core::fmt::Display for OmeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            OmeError::Core(e) => write!(f, "{e}"),
            OmeError::Invalid(what) => write!(f, "invalid OME metadata: {what}"),
            OmeError::Tiff(what) => write!(f, "TIFF error: {what}"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for OmeError {}
