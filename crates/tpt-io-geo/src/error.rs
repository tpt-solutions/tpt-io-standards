//! Error type for the Geo parser, built on [`tpt_io_core::ParseError`].

#[cfg(not(any(feature = "alloc", feature = "std")))]
compile_error!(
    "tpt-io-geo requires the `alloc` or `std` feature: its error type owns structured parse data."
);

use tpt_io_core::ParseError;

/// Errors produced while parsing Geo data.
///
/// The `Core` variant wraps a [`tpt_io_core::ParseError`] carrying the byte
/// offset, expected-vs-actual token, spec reference, and surrounding bytes.
/// Format-specific failure modes are added as additional variants as parsers
/// land (see `todo.md`).
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum GeoError {
    /// A structural error reported by the shared core.
    Core(ParseError),
    /// A Geo-specific feature or construct that is not yet supported.
    Unsupported(&'static str),
}

impl From<ParseError> for GeoError {
    fn from(e: ParseError) -> Self {
        GeoError::Core(e)
    }
}

impl core::fmt::Display for GeoError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            GeoError::Core(e) => write!(f, "{e}"),
            GeoError::Unsupported(feat) => write!(f, "Geo unsupported feature: {feat}"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for GeoError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn core_error_converts() {
        let e = GeoError::from(ParseError::eof(0));
        assert!(matches!(e, GeoError::Core(_)));
    }
}
