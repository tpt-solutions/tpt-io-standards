//! Error type for the TIFF reader, built on [`tpt_io_core::ParseError`].

#[cfg(not(any(feature = "alloc", feature = "std")))]
compile_error!("tpt-io-tiff requires the `alloc` or `std` feature: its error type owns text.");

use alloc::string::String;

use tpt_io_core::ParseError;

/// Errors produced while reading or writing a TIFF file.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum TiffError {
    /// A malformed header, IFD or strip, with its byte offset.
    Core(ParseError),
    /// A feature this crate does not decode (tiles, PackBits, Deflate, BigTIFF, and so on).
    Unsupported(String),
    /// The image description is inconsistent (for example the pixel count does not match the size).
    Invalid(String),
}

impl From<ParseError> for TiffError {
    fn from(e: ParseError) -> Self {
        TiffError::Core(e)
    }
}

impl core::fmt::Display for TiffError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            TiffError::Core(e) => write!(f, "{e}"),
            TiffError::Unsupported(what) => write!(f, "unsupported TIFF feature: {what}"),
            TiffError::Invalid(what) => write!(f, "invalid TIFF: {what}"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for TiffError {}
