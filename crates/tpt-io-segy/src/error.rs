//! Error type for the SEG-Y reader, built on [`tpt_io_core::ParseError`].

#[cfg(not(any(feature = "alloc", feature = "std")))]
compile_error!("tpt-io-segy requires the `alloc` or `std` feature: its error type owns text.");

use alloc::string::String;

use tpt_io_core::ParseError;

/// Errors produced while reading a SEG-Y file.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum SegyError {
    /// The file is shorter than its headers or a trace declares, with its byte offset.
    Core(ParseError),
    /// A sample format this crate does not decode.
    Unsupported(String),
}

impl From<ParseError> for SegyError {
    fn from(e: ParseError) -> Self {
        SegyError::Core(e)
    }
}

impl core::fmt::Display for SegyError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            SegyError::Core(e) => write!(f, "{e}"),
            SegyError::Unsupported(what) => write!(f, "unsupported SEG-Y sample format: {what}"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for SegyError {}
