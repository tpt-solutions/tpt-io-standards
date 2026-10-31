//! Error type for the OpenDSS reader, built on [`tpt_io_core::ParseError`].

#[cfg(not(any(feature = "alloc", feature = "std")))]
compile_error!("tpt-io-opendss requires the `alloc` or `std` feature: its error type owns text.");

use tpt_io_core::ParseError;

/// Errors produced while reading an OpenDSS script.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum OpenDssError {
    /// An unterminated quote or bracket, with the byte offset of the line.
    Core(ParseError),
}

impl From<ParseError> for OpenDssError {
    fn from(e: ParseError) -> Self {
        OpenDssError::Core(e)
    }
}

impl core::fmt::Display for OpenDssError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            OpenDssError::Core(e) => write!(f, "{e}"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for OpenDssError {}
