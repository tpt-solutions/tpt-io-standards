//! Error type for the GDSII reader, built on [`tpt_io_core::ParseError`].

#[cfg(not(any(feature = "alloc", feature = "std")))]
compile_error!("tpt-io-gds requires the `alloc` or `std` feature: its error type owns text.");

use alloc::string::String;

use tpt_io_core::ParseError;

/// Errors produced while reading or writing a GDSII stream.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum GdsError {
    /// A record whose length or data type is inconsistent, with its byte offset.
    Core(ParseError),
    /// The records do not form a valid library (missing `ENDLIB`, element outside a structure, …).
    Invalid(String),
}

impl From<ParseError> for GdsError {
    fn from(e: ParseError) -> Self {
        GdsError::Core(e)
    }
}

impl core::fmt::Display for GdsError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            GdsError::Core(e) => write!(f, "{e}"),
            GdsError::Invalid(what) => write!(f, "invalid GDSII stream: {what}"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for GdsError {}
