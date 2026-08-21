//! Error type for the DICOM parser, built on [`tpt_io_core::ParseError`].

#[cfg(not(any(feature = "alloc", feature = "std")))]
compile_error!("tpt-io-dicom requires the `alloc` or `std` feature: its error type owns structured parse data.");

use tpt_io_core::ParseError;

/// Errors produced while parsing DICOM data.
///
/// The `Core` variant wraps a [`tpt_io_core::ParseError`] carrying the byte
/// offset, expected-vs-actual token, spec reference, and surrounding bytes.
/// Format-specific failure modes are added as additional variants as parsers
/// land (see `todo.md`).
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum DicomError {
    /// A structural error reported by the shared core.
    Core(ParseError),
    /// A DICOM-specific feature or construct that is not yet supported.
    Unsupported(&'static str),
}

impl From<ParseError> for DicomError {
    fn from(e: ParseError) -> Self {
        DicomError::Core(e)
    }
}

impl core::fmt::Display for DicomError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            DicomError::Core(e) => write!(f, "{e}"),
            DicomError::Unsupported(feat) => write!(f, "DICOM unsupported feature: {feat}"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for DicomError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn core_error_converts() {
        let e = DicomError::from(ParseError::eof(0));
        assert!(matches!(e, DicomError::Core(_)));
    }
}
