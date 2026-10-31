//! Error type for the molfile reader, built on [`tpt_io_core::ParseError`].

#[cfg(not(any(feature = "alloc", feature = "std")))]
compile_error!("tpt-io-chem requires the `alloc` or `std` feature: its error type owns text.");

use alloc::string::String;

use tpt_io_core::ParseError;

/// Errors produced while reading a molfile or SDF file.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ChemError {
    /// A malformed count, coordinate or index, with its byte offset.
    Core(ParseError),
    /// The record is structurally incomplete (no counts line, too few atom lines, bad indices).
    Invalid(String),
}

impl From<ParseError> for ChemError {
    fn from(e: ParseError) -> Self {
        ChemError::Core(e)
    }
}

impl core::fmt::Display for ChemError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            ChemError::Core(e) => write!(f, "{e}"),
            ChemError::Invalid(what) => write!(f, "invalid molfile: {what}"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for ChemError {}
