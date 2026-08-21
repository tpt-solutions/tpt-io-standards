//! Error type for the EDIF parser, built on [`tpt_io_core::ParseError`].

#[cfg(not(any(feature = "alloc", feature = "std")))]
compile_error!(
    "tpt-io-edif requires the `alloc` or `std` feature: its error type owns structured parse data."
);

use tpt_io_core::ParseError;

/// Errors produced while parsing EDIF data.
///
/// The `Core` variant wraps a [`tpt_io_core::ParseError`] carrying the byte
/// offset, expected-vs-actual token, spec reference, and surrounding bytes.
/// Format-specific failure modes are added as additional variants as parsers
/// land (see `todo.md`).
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum EdifError {
    /// A structural error reported by the shared core.
    Core(ParseError),
    /// A EDIF-specific feature or construct that is not yet supported.
    Unsupported(&'static str),
}

impl From<ParseError> for EdifError {
    fn from(e: ParseError) -> Self {
        EdifError::Core(e)
    }
}

impl core::fmt::Display for EdifError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            EdifError::Core(e) => write!(f, "{e}"),
            EdifError::Unsupported(feat) => write!(f, "EDIF unsupported feature: {feat}"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for EdifError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn core_error_converts() {
        let e = EdifError::from(ParseError::eof(0));
        assert!(matches!(e, EdifError::Core(_)));
    }
}
