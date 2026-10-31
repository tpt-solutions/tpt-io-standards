//! Error type for the EDF/BDF reader, built on [`tpt_io_core::ParseError`].

#[cfg(not(any(feature = "alloc", feature = "std")))]
compile_error!("tpt-io-edf requires the `alloc` or `std` feature: its error type owns text.");

use alloc::string::String;

use tpt_io_core::ParseError;

/// Errors produced while reading or writing an EDF or BDF file.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum EdfError {
    /// A header field that is not a number, or a file shorter than its header declares.
    Core(ParseError),
    /// The file is structurally inconsistent (for example, sample counts that do not match the records).
    Invalid(String),
    /// A request this crate does not support (for example, writing BDF).
    Unsupported(String),
}

impl From<ParseError> for EdfError {
    fn from(e: ParseError) -> Self {
        EdfError::Core(e)
    }
}

impl core::fmt::Display for EdfError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            EdfError::Core(e) => write!(f, "{e}"),
            EdfError::Invalid(what) => write!(f, "invalid EDF file: {what}"),
            EdfError::Unsupported(what) => write!(f, "unsupported EDF request: {what}"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for EdfError {}
