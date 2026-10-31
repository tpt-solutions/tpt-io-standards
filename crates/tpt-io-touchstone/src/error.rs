//! Error type for the Touchstone reader, built on [`tpt_io_core::ParseError`].

#[cfg(not(any(feature = "alloc", feature = "std")))]
compile_error!("tpt-io-touchstone requires the `alloc` or `std` feature: its error type owns text.");

use alloc::string::String;

use tpt_io_core::ParseError;

/// Errors produced while reading or writing a Touchstone file.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum TouchstoneError {
    /// A malformed number or option, with its byte offset.
    Core(ParseError),
    /// A data row that does not hold a whole frequency and matrix.
    Invalid(String),
}

impl From<ParseError> for TouchstoneError {
    fn from(e: ParseError) -> Self {
        TouchstoneError::Core(e)
    }
}

impl core::fmt::Display for TouchstoneError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            TouchstoneError::Core(e) => write!(f, "{e}"),
            TouchstoneError::Invalid(what) => write!(f, "invalid Touchstone data: {what}"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for TouchstoneError {}
