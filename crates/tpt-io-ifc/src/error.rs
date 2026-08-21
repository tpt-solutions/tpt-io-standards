//! Error type for the IFC parser, built on [`tpt_io_core::ParseError`].

#[cfg(not(any(feature = "alloc", feature = "std")))]
compile_error!(
    "tpt-io-ifc requires the `alloc` or `std` feature: its error type owns structured parse data."
);

use tpt_io_core::ParseError;

/// Errors produced while parsing IFC data.
///
/// The `Core` variant wraps a [`tpt_io_core::ParseError`] carrying the byte
/// offset, expected-vs-actual token, spec reference, and surrounding bytes.
/// Format-specific failure modes are added as additional variants as parsers
/// land (see `todo.md`).
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum IfcError {
    /// A structural error reported by the shared core.
    Core(ParseError),
    /// A IFC-specific feature or construct that is not yet supported.
    Unsupported(&'static str),
}

impl From<ParseError> for IfcError {
    fn from(e: ParseError) -> Self {
        IfcError::Core(e)
    }
}

impl core::fmt::Display for IfcError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            IfcError::Core(e) => write!(f, "{e}"),
            IfcError::Unsupported(feat) => write!(f, "IFC unsupported feature: {feat}"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for IfcError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn core_error_converts() {
        let e = IfcError::from(ParseError::eof(0));
        assert!(matches!(e, IfcError::Core(_)));
    }
}
