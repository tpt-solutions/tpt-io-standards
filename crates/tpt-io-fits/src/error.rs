//! Error type for the FITS reader, built on [`tpt_io_core::ParseError`].

#[cfg(not(any(feature = "alloc", feature = "std")))]
compile_error!("tpt-io-fits requires the `alloc` or `std` feature: its error type owns text.");

use alloc::string::String;

use tpt_io_core::ParseError;

/// Errors produced while reading, writing or decoding FITS data.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum FitsError {
    /// A structural error (a truncated block, a malformed card), with its byte offset.
    Core(ParseError),
    /// The file or HDU parsed but cannot be used as requested.
    Invalid(String),
    /// A `BITPIX` value this crate does not decode.
    UnsupportedBitpix(i64),
}

impl From<ParseError> for FitsError {
    fn from(e: ParseError) -> Self {
        FitsError::Core(e)
    }
}

impl core::fmt::Display for FitsError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            FitsError::Core(e) => write!(f, "{e}"),
            FitsError::Invalid(what) => write!(f, "invalid FITS data: {what}"),
            FitsError::UnsupportedBitpix(b) => write!(f, "unsupported BITPIX {b}"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for FitsError {}
