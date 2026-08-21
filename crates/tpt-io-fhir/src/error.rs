//! Error type for the FHIR parser, built on [`tpt_io_core::ParseError`].

#[cfg(not(any(feature = "alloc", feature = "std")))]
compile_error!(
    "tpt-io-fhir requires the `alloc` or `std` feature: its error type owns structured parse data."
);

use tpt_io_core::ParseError;

/// Errors produced while parsing FHIR data.
///
/// The `Core` variant wraps a [`tpt_io_core::ParseError`] carrying the byte
/// offset, expected-vs-actual token, spec reference, and surrounding bytes.
/// Format-specific failure modes are added as additional variants as parsers
/// land (see `todo.md`).
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum FhirError {
    /// A structural error reported by the shared core.
    Core(ParseError),
    /// A FHIR-specific feature or construct that is not yet supported.
    Unsupported(&'static str),
}

impl From<ParseError> for FhirError {
    fn from(e: ParseError) -> Self {
        FhirError::Core(e)
    }
}

impl core::fmt::Display for FhirError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            FhirError::Core(e) => write!(f, "{e}"),
            FhirError::Unsupported(feat) => write!(f, "FHIR unsupported feature: {feat}"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for FhirError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn core_error_converts() {
        let e = FhirError::from(ParseError::eof(0));
        assert!(matches!(e, FhirError::Core(_)));
    }
}
