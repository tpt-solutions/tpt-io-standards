//! Error type for the WellLog parser, built on [`tpt_io_core::ParseError`].

#[cfg(not(any(feature = "alloc", feature = "std")))]
compile_error!("tpt-io-welllog requires the `alloc` or `std` feature: its error type owns structured parse data.");

use alloc::string::String;
use tpt_io_core::ParseError;

/// Errors produced while parsing WellLog (LAS / DLIS) data.
///
/// The `Core` variant wraps a [`tpt_io_core::ParseError`] carrying the byte
/// offset, expected-vs-actual token, spec reference, and surrounding bytes.
/// The remaining variants cover LAS-specific structural failures.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum WellLogError {
    /// A structural error reported by the shared core.
    Core(ParseError),
    /// A required LAS section (`~V`, `~W`, `~C`, `~A`, …) was missing.
    MissingSection(&'static str),
    /// A numeric field could not be parsed.
    InvalidNumber {
        /// Byte offset of the offending token.
        offset: usize,
        /// The text that failed to parse.
        value: String,
    },
    /// A `~A` data row did not have the expected number of columns.
    InconsistentColumnCount {
        /// Expected column count (number of curves).
        expected: usize,
        /// Columns actually found on the row.
        found: usize,
        /// 1-based data row index.
        line: usize,
    },
    /// A required field (mnemonic) was missing or empty.
    MissingField(&'static str),
    /// A WellLog-specific feature or construct that is not yet supported.
    Unsupported(&'static str),
}

impl From<ParseError> for WellLogError {
    fn from(e: ParseError) -> Self {
        WellLogError::Core(e)
    }
}

impl core::fmt::Display for WellLogError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            WellLogError::Core(e) => write!(f, "{e}"),
            WellLogError::MissingSection(s) => write!(f, "missing required LAS section ~{s}"),
            WellLogError::InvalidNumber { offset, value } => {
                write!(f, "invalid number {value:?} at byte {offset}")
            }
            WellLogError::InconsistentColumnCount { expected, found, line } => {
                write!(f, "data row {line} has {found} columns, expected {expected}")
            }
            WellLogError::MissingField(field) => write!(f, "missing required field {field}"),
            WellLogError::Unsupported(feat) => {
                write!(f, "WellLog unsupported feature: {feat}")
            }
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for WellLogError {}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(not(feature = "std"))]
    use alloc::string::ToString;

    #[test]
    fn core_error_converts() {
        let e = WellLogError::from(ParseError::eof(0));
        assert!(matches!(e, WellLogError::Core(_)));
    }

    #[test]
    fn display_invalid_number() {
        let e = WellLogError::InvalidNumber { offset: 42, value: "NaN".into() };
        assert_eq!(e.to_string(), "invalid number \"NaN\" at byte 42");
    }
}
