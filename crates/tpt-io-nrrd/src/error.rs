//! Error type for the NRRD reader, built on [`tpt_io_core::ParseError`].

#[cfg(not(any(feature = "alloc", feature = "std")))]
compile_error!("tpt-io-nrrd requires the `alloc` or `std` feature: its error type owns text.");

use alloc::string::String;

use tpt_io_core::ParseError;

/// Errors produced while reading or decoding an NRRD file.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum NrrdError {
    /// A malformed header line or missing magic, with its byte offset.
    Core(ParseError),
    /// An encoding or data location this crate does not read.
    Unsupported(String),
    /// The header and data disagree (for example, the data is shorter than its sizes).
    Invalid(String),
}

impl From<ParseError> for NrrdError {
    fn from(e: ParseError) -> Self {
        NrrdError::Core(e)
    }
}

impl core::fmt::Display for NrrdError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            NrrdError::Core(e) => write!(f, "{e}"),
            NrrdError::Unsupported(what) => write!(f, "unsupported NRRD feature: {what}"),
            NrrdError::Invalid(what) => write!(f, "invalid NRRD file: {what}"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for NrrdError {}
