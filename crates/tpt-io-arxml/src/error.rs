//! Error type for the ARXML reader, built on [`tpt_io_core::ParseError`].

#[cfg(not(any(feature = "alloc", feature = "std")))]
compile_error!("tpt-io-arxml requires the `alloc` or `std` feature: its error type owns text.");

use alloc::string::String;

use tpt_io_core::ParseError;

/// Errors produced while reading or validating an ARXML file.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ArxmlError {
    /// Malformed XML or a missing `SHORT-NAME`, with its byte offset.
    Core(ParseError),
    /// The file parsed but breaks a naming rule.
    Invalid(String),
}

impl From<ParseError> for ArxmlError {
    fn from(e: ParseError) -> Self {
        ArxmlError::Core(e)
    }
}

impl core::fmt::Display for ArxmlError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            ArxmlError::Core(e) => write!(f, "{e}"),
            ArxmlError::Invalid(what) => write!(f, "invalid ARXML file: {what}"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for ArxmlError {}
