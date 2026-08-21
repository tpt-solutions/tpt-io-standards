//! Errors produced while parsing DICOM data (PS3.5 / PS3.6 / PS3.7 / PS3.10).

#[cfg(not(any(feature = "alloc", feature = "std")))]
compile_error!("tpt-io-dicom requires the `alloc` or `std` feature: its error type owns structured parse data.");

use alloc::string::String;
use tpt_io_core::ParseError;

/// Convenience alias for results that fail with [`DicomError`].
pub type Result<T> = core::result::Result<T, DicomError>;

/// Specification references used throughout the DICOM parser.
pub mod refs {
    use tpt_io_core::SpecReference;

    /// PS3.5 — Data Structures and Encoding.
    pub const PS3_5: SpecReference = SpecReference::new("DICOM", "PS3.5");
    /// PS3.6 — Data Dictionary.
    pub const PS3_6: SpecReference = SpecReference::new("DICOM", "PS3.6");
    /// PS3.7 — Message Exchange.
    pub const PS3_7: SpecReference = SpecReference::new("DICOM", "PS3.7");
    /// PS3.10 — Media Storage and File Format.
    pub const PS3_10: SpecReference = SpecReference::new("DICOM", "PS3.10");
}

/// Errors produced while parsing DICOM data.
///
/// The `Core` variant wraps a [`tpt_io_core::ParseError`] carrying the byte
/// offset, expected-vs-actual token, spec reference, and surrounding bytes.
/// Format-specific failure modes are added as dedicated variants.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum DicomError {
    /// A structural error reported by the shared core.
    Core(ParseError),
    /// The input does not start with a valid DICOM magic ("DICM") where one
    /// was required.
    MissingDicmPrefix {
        /// Byte offset at which the magic was expected.
        offset: usize,
    },
    /// A transfer syntax that this parser does not support
    /// (e.g. deflated or JPEG 2000 encoded without a codec backend).
    UnsupportedTransferSyntax(String),
    /// A required element (e.g. `(0002,0010)` Transfer Syntax UID) is absent.
    MissingRequiredElement {
        /// The missing tag as `(gggg,eeee)`.
        tag: String,
    },
    /// A value did not match the VR/VM constraints of its tag.
    InvalidValue {
        /// Byte offset where the invalid value begins.
        offset: usize,
        /// Human-readable reason.
        reason: String,
    },
    /// A DICOM-specific construct that is not yet supported.
    Unsupported(&'static str),
}

impl From<ParseError> for DicomError {
    fn from(e: ParseError) -> Self {
        DicomError::Core(e)
    }
}

impl DicomError {
    /// Wrap a core [`ParseError`] and attach surrounding-bytes context.
    pub fn core_with_context(e: ParseError, input: &[u8]) -> Self {
        DicomError::Core(e.with_context_from(input, 16))
    }

    /// Build an [`DicomError::InvalidValue`] from parts.
    pub fn invalid_value(offset: usize, reason: impl Into<String>) -> Self {
        DicomError::InvalidValue { offset, reason: reason.into() }
    }

    /// Byte offset associated with this error, when known.
    pub fn offset(&self) -> Option<usize> {
        match self {
            DicomError::Core(e) => Some(e.offset),
            DicomError::MissingDicmPrefix { offset } => Some(*offset),
            DicomError::InvalidValue { offset, .. } => Some(*offset),
            _ => None,
        }
    }
}

impl core::fmt::Display for DicomError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            DicomError::Core(e) => write!(f, "{e}"),
            DicomError::MissingDicmPrefix { offset } => {
                write!(f, "missing \"DICM\" magic at byte {offset} [PS3.10 §7.1]")
            }
            DicomError::UnsupportedTransferSyntax(uid) => {
                write!(f, "unsupported transfer syntax UID {uid} [PS3.5 §10]")
            }
            DicomError::MissingRequiredElement { tag } => {
                write!(f, "missing required element {tag}")
            }
            DicomError::InvalidValue { offset, reason } => {
                write!(f, "invalid value at byte {offset}: {reason}")
            }
            DicomError::Unsupported(feat) => write!(f, "DICOM unsupported feature: {feat}"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for DicomError {}

/// Convert a failure reason into a DICOM error at `offset`.
#[allow(dead_code)]
pub(crate) fn custom(offset: usize, reason: alloc::string::String) -> DicomError {
    DicomError::Core(ParseError::custom(offset, reason))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn core_error_converts() {
        let e = DicomError::from(ParseError::eof(0));
        assert!(matches!(e, DicomError::Core(_)));
        assert_eq!(e.offset(), Some(0));
    }

    #[test]
    fn display_includes_spec_reference() {
        let e = DicomError::Core(ParseError::eof(7).with_spec(refs::PS3_10));
        let msg = e.to_string();
        assert!(msg.contains("PS3.10"), "unexpected message: {msg}");
    }

    #[test]
    fn display_missing_dicm() {
        let e = DicomError::MissingDicmPrefix { offset: 128 };
        assert_eq!(e.to_string(), "missing \"DICM\" magic at byte 128 [PS3.10 §7.1]");
    }
}
