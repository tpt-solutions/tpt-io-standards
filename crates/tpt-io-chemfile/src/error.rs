//! Error type shared by the structure readers, built on [`tpt_io_core::ParseError`].

#[cfg(not(any(feature = "alloc", feature = "std")))]
compile_error!("tpt-io-chemfile requires the `alloc` or `std` feature: its error type owns text.");

use alloc::string::String;

use tpt_io_core::ParseError;

/// Errors produced while reading or writing a structure file.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ChemError {
    /// A malformed line or number, with its byte offset.
    Core(ParseError),
    /// The file parsed but its counts or sections disagree.
    Invalid(String),
}

impl From<ParseError> for ChemError {
    fn from(e: ParseError) -> Self {
        ChemError::Core(e)
    }
}

impl core::fmt::Display for ChemError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            ChemError::Core(e) => write!(f, "{e}"),
            ChemError::Invalid(what) => write!(f, "invalid structure file: {what}"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for ChemError {}

/// Parse a decimal number from a token, reporting the byte offset on failure.
pub(crate) fn number(tok: &str, at: usize) -> Result<f64, ChemError> {
    tok.parse::<f64>().map_err(|_| ParseError::custom(at, "expected a number").into())
}

/// Split a text into its lines (accepting `\n` or `\r\n`).
pub(crate) fn lines(input: &[u8]) -> Result<alloc::vec::Vec<&str>, ChemError> {
    core::str::from_utf8(input)
        .map(|s| s.lines().collect())
        .map_err(|e| ParseError::custom(e.valid_up_to(), "file is not valid UTF-8").into())
}
