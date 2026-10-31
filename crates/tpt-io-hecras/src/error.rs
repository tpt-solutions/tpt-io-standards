//! Error type for the HEC-RAS reader, built on [`tpt_io_core::ParseError`].

#[cfg(not(any(feature = "alloc", feature = "std")))]
compile_error!("tpt-io-hecras requires the `alloc` or `std` feature: its error type owns text.");

use alloc::string::String;

use tpt_io_core::ParseError;

/// Errors produced while reading a HEC-RAS geometry file.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum HecRasError {
    /// A number that does not parse, or a count that runs past the end of the file, with its byte offset.
    Core(ParseError),
    /// The file is structurally inconsistent (a cross section outside a reach, too few points).
    Invalid(String),
}

impl From<ParseError> for HecRasError {
    fn from(e: ParseError) -> Self {
        HecRasError::Core(e)
    }
}

impl core::fmt::Display for HecRasError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            HecRasError::Core(e) => write!(f, "{e}"),
            HecRasError::Invalid(what) => write!(f, "invalid HEC-RAS geometry: {what}"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for HecRasError {}
