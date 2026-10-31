//! Error type for the AutomationML reader, built on [`tpt_io_core::ParseError`].

#[cfg(not(any(feature = "alloc", feature = "std")))]
compile_error!(
    "tpt-io-automationml requires the `alloc` or `std` feature: its error type owns text."
);

use tpt_io_core::ParseError;

/// Errors produced while reading an AutomationML (CAEX) file.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum AutomationMlError {
    /// Malformed XML, or the root is not `CAEXFile`, with its byte offset.
    Core(ParseError),
}

impl From<ParseError> for AutomationMlError {
    fn from(e: ParseError) -> Self {
        AutomationMlError::Core(e)
    }
}

impl core::fmt::Display for AutomationMlError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            AutomationMlError::Core(e) => write!(f, "{e}"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for AutomationMlError {}
