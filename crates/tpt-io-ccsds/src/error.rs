//! Error type for the CCSDS readers, built on [`tpt_io_core::ParseError`].

#[cfg(not(any(feature = "alloc", feature = "std")))]
compile_error!("tpt-io-ccsds requires the `alloc` or `std` feature: its error type owns text.");

use alloc::string::String;

use tpt_io_core::ParseError;

/// Errors produced while reading, writing or validating CCSDS messages.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum CcsdsError {
    /// A line that does not fit the KVN grammar, with its byte offset.
    Core(ParseError),
    /// The message parsed but breaks a rule (for example, a missing version or a non-numeric field).
    Invalid(String),
}

impl From<ParseError> for CcsdsError {
    fn from(e: ParseError) -> Self {
        CcsdsError::Core(e)
    }
}

impl core::fmt::Display for CcsdsError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            CcsdsError::Core(e) => write!(f, "{e}"),
            CcsdsError::Invalid(what) => write!(f, "invalid CCSDS message: {what}"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for CcsdsError {}
