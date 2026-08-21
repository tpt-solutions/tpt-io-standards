//! Error type for the CityGML parser, built on [`tpt_io_core::ParseError`].

#[cfg(not(any(feature = "alloc", feature = "std")))]
compile_error!("tpt-io-citygml requires the `alloc` or `std` feature: its error type owns structured parse data.");

use tpt_io_core::ParseError;

/// Errors produced while parsing CityGML data.
///
/// The `Core` variant wraps a [`tpt_io_core::ParseError`] carrying the byte
/// offset, expected-vs-actual token, spec reference, and surrounding bytes.
/// Format-specific failure modes are added as additional variants as parsers
/// land (see `todo.md`).
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum CityGmlError {
    /// A structural error reported by the shared core.
    Core(ParseError),
    /// A CityGML-specific feature or construct that is not yet supported.
    Unsupported(&'static str),
}

impl From<ParseError> for CityGmlError {
    fn from(e: ParseError) -> Self {
        CityGmlError::Core(e)
    }
}

impl core::fmt::Display for CityGmlError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            CityGmlError::Core(e) => write!(f, "{e}"),
            CityGmlError::Unsupported(feat) => write!(f, "CityGML unsupported feature: {feat}"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for CityGmlError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn core_error_converts() {
        let e = CityGmlError::from(ParseError::eof(0));
        assert!(matches!(e, CityGmlError::Core(_)));
    }
}
