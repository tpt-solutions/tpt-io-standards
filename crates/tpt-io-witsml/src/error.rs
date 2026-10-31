//! Error type for the WITSML reader, built on [`tpt_io_core::ParseError`].

#[cfg(not(any(feature = "alloc", feature = "std")))]
compile_error!("tpt-io-witsml requires the `alloc` or `std` feature: its error type owns text.");

use alloc::string::String;

use tpt_io_core::ParseError;

/// Errors produced while reading a WITSML document.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum WitsmlError {
    /// Malformed XML, or a number that does not parse, with its byte offset.
    Core(ParseError),
    /// A station without a measured depth, inclination or azimuth.
    Invalid(String),
}

impl From<ParseError> for WitsmlError {
    fn from(e: ParseError) -> Self {
        WitsmlError::Core(e)
    }
}

impl core::fmt::Display for WitsmlError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            WitsmlError::Core(e) => write!(f, "{e}"),
            WitsmlError::Invalid(what) => write!(f, "invalid WITSML trajectory: {what}"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for WitsmlError {}
