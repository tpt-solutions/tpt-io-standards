//! Error type for the CIF reader, built on [`tpt_io_core::ParseError`].

#[cfg(not(any(feature = "alloc", feature = "std")))]
compile_error!("tpt-io-cif requires the `alloc` or `std` feature: its error type owns text.");

use alloc::string::String;

use tpt_io_core::ParseError;

/// Errors produced while reading or validating a CIF file.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum CifError {
    /// A syntax error (unterminated quote or text field, a value with no tag), with its byte offset.
    Core(ParseError),
    /// The file parsed but breaks a rule (for example, a loop whose values do not fill its rows).
    Invalid(String),
}

impl From<ParseError> for CifError {
    fn from(e: ParseError) -> Self {
        CifError::Core(e)
    }
}

impl core::fmt::Display for CifError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            CifError::Core(e) => write!(f, "{e}"),
            CifError::Invalid(what) => write!(f, "invalid CIF data: {what}"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for CifError {}
