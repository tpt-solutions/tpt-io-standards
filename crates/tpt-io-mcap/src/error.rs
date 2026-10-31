//! Error type for the MCAP reader, built on [`tpt_io_core::ParseError`].

#[cfg(not(any(feature = "alloc", feature = "std")))]
compile_error!("tpt-io-mcap requires the `alloc` or `std` feature: its error type owns text.");

use tpt_io_core::ParseError;

/// Errors produced while reading or writing an MCAP file.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum McapError {
    /// A structural error, with its byte offset.
    Core(ParseError),
    /// A feature this reader does not implement, such as a compressed chunk.
    Unsupported(&'static str),
}

impl From<ParseError> for McapError {
    fn from(e: ParseError) -> Self {
        McapError::Core(e)
    }
}

impl core::fmt::Display for McapError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            McapError::Core(e) => write!(f, "{e}"),
            McapError::Unsupported(what) => write!(f, "unsupported MCAP feature: {what}"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for McapError {}
