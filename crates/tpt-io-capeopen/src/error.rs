//! Error type for CAPE-OPEN, built on [`tpt_io_core::ParseError`].
//!
//! The `Core` variant wraps a [`tpt_io_core::ParseError`] carrying the byte
//! offset, expected-vs-actual token, spec reference, and surrounding bytes for
//! failures produced by the property-package text codec. The remaining
//! variants cover the pure-Rust data model (component lists, flowsheet
//! topology) and the Windows-only COM interop layer.

#[cfg(not(any(feature = "alloc", feature = "std")))]
compile_error!("tpt-io-capeopen requires the `alloc` or `std` feature: its error type owns structured data.");

use tpt_io_core::ParseError;

/// Errors produced while working with CAPE-OPEN data.
///
/// The `Core` variant wraps a [`tpt_io_core::ParseError`] from the property
/// package text codec. Format-specific failure modes (invalid component lists,
/// broken flowsheet connectivity, COM `HRESULT` failures, …) have their own
/// variants.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum CapeOpenError {
    /// A structural error reported by the shared core (property-package codec).
    Core(ParseError),
    /// A CAPE-OPEN-specific feature or construct that is not yet supported.
    Unsupported(&'static str),
    /// A component list failed validation (duplicate id, empty list, …).
    ComponentList(String),
    /// A property package failed validation (missing components/phases, …).
    PropertyPackage(String),
    /// A flowsheet failed connectivity validation.
    Flowsheet(String),
    /// A COM call returned a failure `HRESULT`.
    Com(String),
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
            CapeOpenError::ComponentList(m) => write!(f, "component list error: {m}"),
            CapeOpenError::PropertyPackage(m) => write!(f, "property package error: {m}"),
            CapeOpenError::Flowsheet(m) => write!(f, "flowsheet error: {m}"),
            CapeOpenError::Com(m) => write!(f, "CAPE-OPEN COM error: {m}"),
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

    #[test]
    fn display_describes_variants() {
        let e = CapeOpenError::Flowsheet("dangling inlet".into());
        assert_eq!(e.to_string(), "flowsheet error: dangling inlet");
    }
}
