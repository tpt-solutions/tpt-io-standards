//! Error type for the Gerber parser, built on [`tpt_io_core::ParseError`].

#[cfg(not(any(feature = "alloc", feature = "std")))]
compile_error!("tpt-io-gerber requires the `alloc` or `std` feature: its error type owns structured parse data.");

use tpt_io_core::ParseError;

/// Errors produced while parsing Gerber data.
///
/// The `Core` variant wraps a [`tpt_io_core::ParseError`] carrying the byte
/// offset, expected-vs-actual token, spec reference, and surrounding bytes.
/// Format-specific failure modes are added as additional variants as parsers
/// land (see `todo.md`).
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum GerberError {
    /// A structural error reported by the shared core.
    Core(ParseError),
    /// A Gerber-specific feature or construct that is not yet supported.
    Unsupported(&'static str),
}

impl From<ParseError> for GerberError {
    fn from(e: ParseError) -> Self {
        GerberError::Core(e)
    }
}

impl core::fmt::Display for GerberError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            GerberError::Core(e) => write!(f, "{e}"),
            GerberError::Unsupported(feat) => write!(f, "Gerber unsupported feature: {feat}"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for GerberError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn core_error_converts() {
        let e = GerberError::from(ParseError::eof(0));
        assert!(matches!(e, GerberError::Core(_)));
    }
}
