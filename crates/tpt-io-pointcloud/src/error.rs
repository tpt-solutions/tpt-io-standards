//! Error type for the LAS reader, built on [`tpt_io_core::ParseError`].

#[cfg(not(any(feature = "alloc", feature = "std")))]
compile_error!("tpt-io-pointcloud requires the `alloc` or `std` feature: its error type owns text.");

use alloc::string::String;

use tpt_io_core::ParseError;

/// Errors produced while reading or writing a LAS file.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum LasError {
    /// A truncated file, or a header field outside its range, with its byte offset.
    Core(ParseError),
    /// A point format or LAS version this crate does not handle (LAZ, formats 4 and up).
    Unsupported(String),
    /// A value that cannot be written (for example, a coordinate that does not fit the scale).
    Invalid(String),
}

impl From<ParseError> for LasError {
    fn from(e: ParseError) -> Self {
        LasError::Core(e)
    }
}

impl core::fmt::Display for LasError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            LasError::Core(e) => write!(f, "{e}"),
            LasError::Unsupported(what) => write!(f, "unsupported LAS feature: {what}"),
            LasError::Invalid(what) => write!(f, "invalid LAS data: {what}"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for LasError {}
