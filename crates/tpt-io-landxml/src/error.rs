//! Error type for the LandXML reader, built on [`tpt_io_core::ParseError`].

#[cfg(not(any(feature = "alloc", feature = "std")))]
compile_error!("tpt-io-landxml requires the `alloc` or `std` feature: its error type owns text.");

use alloc::string::String;

use tpt_io_core::ParseError;

/// Errors produced while reading or validating a LandXML document.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum LandXmlError {
    /// Malformed XML or a malformed number, with its byte offset.
    Core(ParseError),
    /// The document parsed but a face refers to a point that does not exist, or a point is malformed.
    Invalid(String),
}

impl From<ParseError> for LandXmlError {
    fn from(e: ParseError) -> Self {
        LandXmlError::Core(e)
    }
}

impl core::fmt::Display for LandXmlError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            LandXmlError::Core(e) => write!(f, "{e}"),
            LandXmlError::Invalid(what) => write!(f, "invalid LandXML: {what}"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for LandXmlError {}
