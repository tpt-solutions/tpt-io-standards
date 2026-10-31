//! Error type for the GNSS readers, built on [`tpt_io_core::ParseError`].

#[cfg(not(any(feature = "alloc", feature = "std")))]
compile_error!("tpt-io-gnss requires the `alloc` or `std` feature: its error type owns text.");

use alloc::string::String;

use tpt_io_core::ParseError;

/// Errors produced while reading RINEX or SP3 data.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum GnssError {
    /// A malformed number or header line, with its byte offset.
    Core(ParseError),
    /// The file is structurally incomplete (no `END OF HEADER`, an epoch with missing satellites).
    Invalid(String),
}

impl From<ParseError> for GnssError {
    fn from(e: ParseError) -> Self {
        GnssError::Core(e)
    }
}

impl core::fmt::Display for GnssError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            GnssError::Core(e) => write!(f, "{e}"),
            GnssError::Invalid(what) => write!(f, "invalid GNSS file: {what}"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for GnssError {}
