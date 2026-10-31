//! Error type for the AAS reader, built on [`tpt_io_core::ParseError`].

#[cfg(not(any(feature = "alloc", feature = "std")))]
compile_error!("tpt-io-aas requires the `alloc` or `std` feature: its error type owns text.");

use alloc::string::String;

use tpt_io_core::ParseError;

/// Errors produced while reading or validating an AAS environment.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum AasError {
    /// Malformed JSON, with its byte offset.
    Core(ParseError),
    /// The document parsed but is not a usable environment (missing identifiers, duplicates).
    Invalid(String),
}

impl From<ParseError> for AasError {
    fn from(e: ParseError) -> Self {
        AasError::Core(e)
    }
}

impl core::fmt::Display for AasError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            AasError::Core(e) => write!(f, "{e}"),
            AasError::Invalid(what) => write!(f, "invalid AAS environment: {what}"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for AasError {}
