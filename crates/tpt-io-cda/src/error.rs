//! Error type for the CDA reader, built on [`tpt_io_core::ParseError`].

#[cfg(not(any(feature = "alloc", feature = "std")))]
compile_error!("tpt-io-cda requires the `alloc` or `std` feature: its error type owns text.");

use tpt_io_core::ParseError;

/// Errors produced while reading a CDA document.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum CdaError {
    /// Malformed XML, or a root that is not `ClinicalDocument`, with its byte offset.
    Core(ParseError),
}

impl From<ParseError> for CdaError {
    fn from(e: ParseError) -> Self {
        CdaError::Core(e)
    }
}

impl core::fmt::Display for CdaError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            CdaError::Core(e) => write!(f, "{e}"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for CdaError {}
