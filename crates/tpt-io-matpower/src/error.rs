//! Error type for the MATPOWER reader, built on [`tpt_io_core::ParseError`].

#[cfg(not(any(feature = "alloc", feature = "std")))]
compile_error!("tpt-io-matpower requires the `alloc` or `std` feature: its error type owns text.");

use alloc::string::String;

use tpt_io_core::ParseError;

/// Errors produced while reading, writing or validating a MATPOWER case.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum MatpowerError {
    /// A syntax error in the case script, with its byte offset.
    Core(ParseError),
    /// The case parsed but breaks a structural or semantic rule.
    Invalid(String),
}

impl From<ParseError> for MatpowerError {
    fn from(e: ParseError) -> Self {
        MatpowerError::Core(e)
    }
}

impl core::fmt::Display for MatpowerError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            MatpowerError::Core(e) => write!(f, "{e}"),
            MatpowerError::Invalid(what) => write!(f, "invalid MATPOWER case: {what}"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for MatpowerError {}
