//! Error type for the OpenDRIVE reader, built on [`tpt_io_core::ParseError`].

#[cfg(not(any(feature = "alloc", feature = "std")))]
compile_error!("tpt-io-opendrive requires the `alloc` or `std` feature: its error type owns text.");

use alloc::string::String;

use tpt_io_core::ParseError;

/// Errors produced while reading, writing or validating an OpenDRIVE map.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum OpenDriveError {
    /// Malformed XML or a missing or malformed attribute, with its byte offset.
    Core(ParseError),
    /// The map parsed but breaks a rule (for example, a geometry gap).
    Invalid(String),
}

impl From<ParseError> for OpenDriveError {
    fn from(e: ParseError) -> Self {
        OpenDriveError::Core(e)
    }
}

impl core::fmt::Display for OpenDriveError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            OpenDriveError::Core(e) => write!(f, "{e}"),
            OpenDriveError::Invalid(what) => write!(f, "invalid OpenDRIVE map: {what}"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for OpenDriveError {}
