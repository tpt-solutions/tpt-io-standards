//! Error type for the SBML parser, built on [`tpt_io_core::ParseError`].

#[cfg(not(any(feature = "alloc", feature = "std")))]
compile_error!(
    "tpt-io-sbml requires the `alloc` or `std` feature: its error type owns structured parse data."
);

use tpt_io_core::ParseError;

/// Errors produced while parsing SBML data.
///
/// The `Core` variant wraps a [`tpt_io_core::ParseError`] carrying the byte
/// offset, expected-vs-actual token, spec reference, and surrounding bytes.
/// Format-specific failure modes are added as additional variants as parsers
/// land (see `todo.md`).
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum SbmlError {
    /// A structural error reported by the shared core.
    Core(ParseError),
    /// A SBML-specific feature or construct that is not yet supported.
    Unsupported(&'static str),
}

impl From<ParseError> for SbmlError {
    fn from(e: ParseError) -> Self {
        SbmlError::Core(e)
    }
}

impl core::fmt::Display for SbmlError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            SbmlError::Core(e) => write!(f, "{e}"),
            SbmlError::Unsupported(feat) => write!(f, "SBML unsupported feature: {feat}"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for SbmlError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn core_error_converts() {
        let e = SbmlError::from(ParseError::eof(0));
        assert!(matches!(e, SbmlError::Core(_)));
    }
}
