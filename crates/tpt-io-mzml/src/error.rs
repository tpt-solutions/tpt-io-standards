//! Error type for the mzML reader, built on [`tpt_io_core::ParseError`].

#[cfg(not(any(feature = "alloc", feature = "std")))]
compile_error!("tpt-io-mzml requires the `alloc` or `std` feature: its error type owns text.");

use alloc::string::String;

use tpt_io_core::ParseError;

/// Errors produced while reading an mzML file.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum MzmlError {
    /// Malformed XML, base64, or a binary array whose length does not match its precision.
    Core(ParseError),
    /// A binary encoding this crate does not decode (zlib compression, or a precision other than 32/64-bit).
    Unsupported(String),
    /// The document parsed but is not usable (a missing `mzML` root, or a spectrum without an id).
    Invalid(String),
}

impl From<ParseError> for MzmlError {
    fn from(e: ParseError) -> Self {
        MzmlError::Core(e)
    }
}

impl core::fmt::Display for MzmlError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            MzmlError::Core(e) => write!(f, "{e}"),
            MzmlError::Unsupported(what) => write!(f, "unsupported mzML encoding: {what}"),
            MzmlError::Invalid(what) => write!(f, "invalid mzML file: {what}"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for MzmlError {}
