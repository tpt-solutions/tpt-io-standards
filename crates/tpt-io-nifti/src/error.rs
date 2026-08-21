//! Error type for the NIfTI parser, built on [`tpt_io_core::ParseError`].

#[cfg(not(any(feature = "alloc", feature = "std")))]
compile_error!("tpt-io-nifti requires the `alloc` or `std` feature: its error type owns structured parse data.");

use tpt_io_core::ParseError;

/// Errors produced while parsing NIfTI data.
///
/// The `Core` variant wraps a [`tpt_io_core::ParseError`] carrying the byte
/// offset, expected-vs-actual token, spec reference, and surrounding bytes.
/// Format-specific failure modes are added as additional variants as parsers
/// land (see `todo.md`).
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum NiftiError {
    /// A structural error reported by the shared core.
    Core(ParseError),
    /// A NIfTI-specific feature or construct that is not yet supported.
    Unsupported(&'static str),
}

impl From<ParseError> for NiftiError {
    fn from(e: ParseError) -> Self {
        NiftiError::Core(e)
    }
}

impl core::fmt::Display for NiftiError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            NiftiError::Core(e) => write!(f, "{e}"),
            NiftiError::Unsupported(feat) => write!(f, "NIfTI unsupported feature: {feat}"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for NiftiError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn core_error_converts() {
        let e = NiftiError::from(ParseError::eof(0));
        assert!(matches!(e, NiftiError::Core(_)));
    }
}
