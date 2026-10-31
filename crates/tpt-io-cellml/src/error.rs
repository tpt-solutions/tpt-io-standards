//! Error type for the CellML reader, built on [`tpt_io_core::ParseError`].

#[cfg(not(any(feature = "alloc", feature = "std")))]
compile_error!("tpt-io-cellml requires the `alloc` or `std` feature: its error type owns text.");

use alloc::string::String;

use tpt_io_core::ParseError;

/// Errors produced while reading a CellML model.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum CellMlError {
    /// Malformed XML, with its byte offset.
    Core(ParseError),
    /// The root is not a CellML `model`, or a required name is missing.
    Invalid(String),
}

impl From<ParseError> for CellMlError {
    fn from(e: ParseError) -> Self {
        CellMlError::Core(e)
    }
}

impl core::fmt::Display for CellMlError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            CellMlError::Core(e) => write!(f, "{e}"),
            CellMlError::Invalid(what) => write!(f, "invalid CellML model: {what}"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for CellMlError {}
