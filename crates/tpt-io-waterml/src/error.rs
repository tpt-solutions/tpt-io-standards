//! Error type for the WaterML reader, built on [`tpt_io_core::ParseError`].

#[cfg(not(any(feature = "alloc", feature = "std")))]
compile_error!("tpt-io-waterml requires the `alloc` or `std` feature: its error type owns text.");

use alloc::string::String;

use tpt_io_core::ParseError;

/// Errors produced while reading or writing WaterML documents.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum WaterMlError {
    /// Malformed XML or a value that is not a number, with its byte offset.
    Core(ParseError),
    /// The document parsed but lacks something a time series needs.
    Invalid(String),
}

impl From<ParseError> for WaterMlError {
    fn from(e: ParseError) -> Self {
        WaterMlError::Core(e)
    }
}

impl core::fmt::Display for WaterMlError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            WaterMlError::Core(e) => write!(f, "{e}"),
            WaterMlError::Invalid(what) => write!(f, "invalid WaterML document: {what}"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for WaterMlError {}
