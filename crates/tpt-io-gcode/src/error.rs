//! Error type for the G-code reader, built on [`tpt_io_core::ParseError`].

#[cfg(not(any(feature = "alloc", feature = "std")))]
compile_error!("tpt-io-gcode requires the `alloc` or `std` feature: its error type owns text.");

use alloc::string::String;

use tpt_io_core::ParseError;

/// Errors produced while reading G-code.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum GcodeError {
    /// A malformed word or an unterminated comment, with its byte offset.
    Core(ParseError),
    /// A word whose value is not a number, e.g. `X1.2.3`.
    BadNumber { offset: usize, text: String },
}

impl From<ParseError> for GcodeError {
    fn from(e: ParseError) -> Self {
        GcodeError::Core(e)
    }
}

impl core::fmt::Display for GcodeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            GcodeError::Core(e) => write!(f, "{e}"),
            GcodeError::BadNumber { offset, text } => {
                write!(f, "bad G-code number '{text}' at byte {offset}")
            }
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for GcodeError {}
