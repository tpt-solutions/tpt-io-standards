//! Error type for the JCAMP-DX reader, built on [`tpt_io_core::ParseError`].

#[cfg(not(any(feature = "alloc", feature = "std")))]
compile_error!("tpt-io-jcamp requires the `alloc` or `std` feature: its error type owns text.");

use alloc::string::String;

use tpt_io_core::ParseError;

/// Errors produced while reading a JCAMP-DX file.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum JcampError {
    /// A malformed number or label, with its byte offset.
    Core(ParseError),
    /// A data encoding this crate does not decode (for example `SQZ` or `DIF`).
    Unsupported(String),
    /// The file has no data block, or the data does not fill the points it declares.
    Invalid(String),
}

impl From<ParseError> for JcampError {
    fn from(e: ParseError) -> Self {
        JcampError::Core(e)
    }
}

impl core::fmt::Display for JcampError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            JcampError::Core(e) => write!(f, "{e}"),
            JcampError::Unsupported(what) => write!(f, "unsupported JCAMP-DX encoding: {what}"),
            JcampError::Invalid(what) => write!(f, "invalid JCAMP-DX file: {what}"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for JcampError {}
