//! Error type for the PLCopen reader, built on [`tpt_io_core::ParseError`].

#[cfg(not(any(feature = "alloc", feature = "std")))]
compile_error!("tpt-io-plcopen requires the `alloc` or `std` feature: its error type owns text.");

use alloc::string::String;

use tpt_io_core::ParseError;

/// Errors produced while reading a PLCopen project.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum PlcOpenError {
    /// Malformed XML, with its byte offset.
    Core(ParseError),
    /// The document parsed but is not a usable PLCopen project.
    Invalid(String),
}

impl From<ParseError> for PlcOpenError {
    fn from(e: ParseError) -> Self {
        PlcOpenError::Core(e)
    }
}

impl core::fmt::Display for PlcOpenError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            PlcOpenError::Core(e) => write!(f, "{e}"),
            PlcOpenError::Invalid(what) => write!(f, "invalid PLCopen project: {what}"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for PlcOpenError {}
