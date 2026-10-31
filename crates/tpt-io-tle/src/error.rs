//! Error type for the TLE reader, built on [`tpt_io_core::ParseError`].

#[cfg(not(any(feature = "alloc", feature = "std")))]
compile_error!("tpt-io-tle requires the `alloc` or `std` feature: its error type owns text.");

use alloc::string::String;

use tpt_io_core::ParseError;

/// Errors produced while reading, writing or validating TLEs.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum TleError {
    /// A malformed line, with its byte offset.
    Core(ParseError),
    /// A line whose checksum does not match column 69.
    Checksum { satellite: u32, line: u8 },
    /// Lines that parse but describe an impossible orbit.
    Invalid(String),
}

impl From<ParseError> for TleError {
    fn from(e: ParseError) -> Self {
        TleError::Core(e)
    }
}

impl core::fmt::Display for TleError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            TleError::Core(e) => write!(f, "{e}"),
            TleError::Checksum { satellite, line } => {
                write!(f, "checksum mismatch on line {line} of satellite {satellite}")
            }
            TleError::Invalid(what) => write!(f, "invalid TLE: {what}"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for TleError {}
