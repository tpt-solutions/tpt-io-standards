//! Error type for the AIXM reader, built on [`tpt_io_core::ParseError`].

#[cfg(not(any(feature = "alloc", feature = "std")))]
compile_error!("tpt-io-aixm requires the `alloc` or `std` feature: its error type owns text.");

use alloc::string::String;

use tpt_io_core::ParseError;

/// Errors produced while reading AIXM messages.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum AixmError {
    /// Malformed XML, with its byte offset.
    Core(ParseError),
    /// A feature or position that is not usable (for example a non-numeric coordinate).
    Invalid(String),
}

impl From<ParseError> for AixmError {
    fn from(e: ParseError) -> Self {
        AixmError::Core(e)
    }
}

impl core::fmt::Display for AixmError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            AixmError::Core(e) => write!(f, "{e}"),
            AixmError::Invalid(what) => write!(f, "invalid AIXM data: {what}"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for AixmError {}
