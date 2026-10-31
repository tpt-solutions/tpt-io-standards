//! Error type for the S-57 reader, built on [`tpt_io_core::ParseError`].

#[cfg(not(any(feature = "alloc", feature = "std")))]
compile_error!("tpt-io-s57 requires the `alloc` or `std` feature: its error type owns text.");

use alloc::string::String;

use tpt_io_core::ParseError;

/// Errors produced while reading an S-57 exchange file.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum S57Error {
    /// Malformed ISO 8211 framing or field data, with its byte offset.
    Core(ParseError),
    /// A file that is well framed but not a usable chart: missing parameters or records,
    /// or a feature that refers to a vector record the file does not contain.
    Invalid(String),
}

impl From<ParseError> for S57Error {
    fn from(e: ParseError) -> Self {
        S57Error::Core(e)
    }
}

impl core::fmt::Display for S57Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            S57Error::Core(e) => write!(f, "{e}"),
            S57Error::Invalid(what) => write!(f, "invalid S-57 file: {what}"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for S57Error {}
