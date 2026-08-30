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
    /// An invalid Value Representation (VR) code was encountered.
    InvalidVr {
        /// Byte offset in the input.
        offset: usize,
        /// The two bytes that were interpreted as a VR.
        actual: [u8; 2],
    },
    /// An invalid DICOM tag (group, element) was encountered.
    InvalidTag {
        /// Byte offset in the input.
        offset: usize,
        /// The raw group number read.
        group: u16,
        /// The raw element number read.
        element: u16,
    },
    /// A required DICOM tag is missing from the dataset.
    MissingTag {
        /// The tag that was expected, as (group, element).
        expected: (u16, u16),
    },
    /// The transfer syntax UID is unknown or unsupported.
    UnknownTransferSyntax {
        /// The UID string encountered.
        uid: alloc::string::String,
    },
    /// The pixel data is malformed or truncated.
    InvalidPixelData {
        /// Byte offset where the problem was detected.
        offset: usize,
    },
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
            DicomError::InvalidVr { offset, actual } => {
                write!(f, "invalid VR at offset {offset}: {:02x}{:02x}", actual[0], actual[1])
            }
            DicomError::InvalidTag { offset, group, element } => {
                write!(f, "invalid tag at offset {offset}: ({group:04x},{element:04x})")
            }
            DicomError::MissingTag { expected } => {
                write!(f, "missing required tag ({:04x},{:04x})", expected.0, expected.1)
            }
            DicomError::UnknownTransferSyntax { uid } => {
                write!(f, "unknown transfer syntax: {uid}")
            }
            DicomError::InvalidPixelData { offset } => {
                write!(f, "invalid pixel data at offset {offset}")
            }
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

    #[test]
    fn display_vr_error() {
        let e = DicomError::InvalidVr { offset: 42, actual: [0xFF, 0xFF] };
        let s = alloc::format!("{e}");
        assert!(s.contains("invalid VR"));
        assert!(s.contains("42"));
    }

    #[test]
    fn display_missing_tag() {
        let e = DicomError::MissingTag { expected: (0x0008, 0x0016) };
        let s = alloc::format!("{e}");
        assert!(s.contains("0008"));
    }
}
