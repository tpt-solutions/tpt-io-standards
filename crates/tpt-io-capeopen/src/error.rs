//! Error type for the CAPE-OPEN parser, built on [`tpt_io_core::ParseError`].

#[cfg(not(any(feature = "alloc", feature = "std")))]
compile_error!("tpt-io-capeopen requires the `alloc` or `std` feature: its error type owns structured parse data.");

use tpt_io_core::ParseError;

/// Errors produced while parsing CAPE-OPEN data.
///
/// The `Core` variant wraps a [`tpt_io_core::ParseError`] carrying the byte
/// offset, expected-vs-actual token, spec reference, and surrounding bytes.
/// Format-specific failure modes are added as additional variants as parsers
/// land (see `todo.md`).
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum CapeOpenError {
    /// A structural error reported by the shared core.
    Core(ParseError),
    /// A CAPE-OPEN-specific feature or construct that is not yet supported.
    Unsupported(&'static str),
}

impl From<ParseError> for CapeOpenError {
    fn from(e: ParseError) -> Self {
        CapeOpenError::Core(e)
    }
}

impl core::fmt::Display for CapeOpenError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            CapeOpenError::Core(e) => write!(f, "{e}"),
            CapeOpenError::Unsupported(feat) => write!(f, "CAPE-OPEN unsupported feature: {feat}"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for CapeOpenError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn core_error_converts() {
        let e = CapeOpenError::from(ParseError::eof(0));
        assert!(matches!(e, CapeOpenError::Core(_)));
    }
}
