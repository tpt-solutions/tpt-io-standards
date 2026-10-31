//! Error type for the gbXML reader, built on [`tpt_io_core::ParseError`].

#[cfg(not(any(feature = "alloc", feature = "std")))]
compile_error!("tpt-io-gbxml requires the `alloc` or `std` feature: its error type owns text.");

use alloc::string::String;

use tpt_io_core::ParseError;

/// Errors produced while reading a gbXML document.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum GbXmlError {
    /// Malformed XML or a malformed number, with its byte offset.
    Core(ParseError),
    /// The document parsed but a surface or space is missing a required identifier or coordinate.
    Invalid(String),
}

impl From<ParseError> for GbXmlError {
    fn from(e: ParseError) -> Self {
        GbXmlError::Core(e)
    }
}

impl core::fmt::Display for GbXmlError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            GbXmlError::Core(e) => write!(f, "{e}"),
            GbXmlError::Invalid(what) => write!(f, "invalid gbXML: {what}"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for GbXmlError {}
