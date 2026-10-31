//! Error type for the DEXPI reader, built on [`tpt_io_core::ParseError`].

#[cfg(not(any(feature = "alloc", feature = "std")))]
compile_error!("tpt-io-dexpi requires the `alloc` or `std` feature: its error type owns text.");

use tpt_io_core::ParseError;

/// Errors produced while reading a DEXPI document.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum DexpiError {
    /// Malformed XML, with its byte offset.
    Core(ParseError),
}

impl From<ParseError> for DexpiError {
    fn from(e: ParseError) -> Self {
        DexpiError::Core(e)
    }
}

impl core::fmt::Display for DexpiError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            DexpiError::Core(e) => write!(f, "{e}"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for DexpiError {}
