//! Error type for the CityGML parser, built on [`tpt_io_core::ParseError`].

#[cfg(not(any(feature = "alloc", feature = "std")))]
compile_error!("tpt-io-citygml requires the `alloc` or `std` feature: its error type owns structured parse data.");

use alloc::string::String;
use tpt_io_core::ParseError;

/// Errors produced while parsing CityGML 2.0 / 3.0 data or serializing CityJSON.
///
/// The `Core` variant wraps a [`tpt_io_core::ParseError`] carrying the byte
/// offset, expected-vs-actual token, spec reference, and surrounding bytes.
/// The remaining variants cover CityGML/CityJSON-specific structural failures.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum CityGmlError {
    /// A structural error reported by the shared core.
    Core(ParseError),
    /// A CityGML-specific feature or construct that is not yet supported.
    Unsupported(&'static str),
    /// An XML well-formedness or namespace error at `offset`.
    XmlError {
        /// Byte offset where the error occurred.
        offset: usize,
        /// Human-readable reason.
        reason: String,
    },
    /// An element or attribute required by the CityGML schema was missing.
    MissingElement {
        /// Element name that was expected.
        name: &'static str,
        /// Byte offset where it was expected.
        offset: usize,
    },
    /// A LOD (level-of-detail) value was outside the 0–4 range.
    InvalidLod {
        /// The offending LOD value.
        value: u32,
        /// Byte offset of the LOD element.
        offset: usize,
    },
    /// A coordinate reference system string could not be parsed.
    InvalidCrs {
        /// The offending CRS identifier.
        crs: String,
        /// Byte offset where it appeared.
        offset: usize,
    },
    /// A numeric value (e.g. a measurement) could not be parsed.
    InvalidNumber {
        /// The offending value text.
        value: String,
        /// Byte offset where it appeared.
        offset: usize,
    },
    /// The input ended before the parser expected more data.
    UnexpectedEof,
    /// A round-trip serialization produced an internally inconsistent value.
    RoundTrip(String),
    /// Any other format-specific failure with a human-readable reason.
    Custom(String),
}

impl CityGmlError {
    /// Build an `XmlError`.
    pub fn xml_error(offset: usize, reason: impl Into<String>) -> Self {
        CityGmlError::XmlError { offset, reason: reason.into() }
    }

    /// Build a `MissingElement` error.
    pub fn missing_element(name: &'static str, offset: usize) -> Self {
        CityGmlError::MissingElement { name, offset }
    }

    /// Build an `InvalidLod` error.
    pub fn invalid_lod(value: u32, offset: usize) -> Self {
        CityGmlError::InvalidLod { value, offset }
    }

    /// Build an `InvalidCrs` error.
    pub fn invalid_crs(crs: impl Into<String>, offset: usize) -> Self {
        CityGmlError::InvalidCrs { crs: crs.into(), offset }
    }

    /// Build an `InvalidNumber` error.
    pub fn invalid_number(value: impl Into<String>, offset: usize) -> Self {
        CityGmlError::InvalidNumber { value: value.into(), offset }
    }

    /// Build a generic `RoundTrip` error.
    pub fn round_trip(msg: impl Into<String>) -> Self {
        CityGmlError::RoundTrip(msg.into())
    }

    /// Build a generic `Custom` error.
    pub fn custom(msg: impl Into<String>) -> Self {
        CityGmlError::Custom(msg.into())
    }
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
            CityGmlError::XmlError { offset, reason } => {
                write!(f, "XML error at offset {offset}: {reason}")
            }
            CityGmlError::MissingElement { name, offset } => {
                write!(f, "missing required element {name:?} at offset {offset}")
            }
            CityGmlError::InvalidLod { value, offset } => {
                write!(f, "invalid LOD level {value} at offset {offset} (expected 0–4)")
            }
            CityGmlError::InvalidCrs { crs, offset } => {
                write!(f, "invalid CRS {crs:?} at offset {offset}")
            }
            CityGmlError::InvalidNumber { value, offset } => {
                write!(f, "invalid number {value:?} at offset {offset}")
            }
            CityGmlError::UnexpectedEof => write!(f, "unexpected end of input"),
            CityGmlError::RoundTrip(msg) => write!(f, "round-trip error: {msg}"),
            CityGmlError::Custom(msg) => write!(f, "{msg}"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for CityGmlError {}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(not(feature = "std"))]
    use alloc::string::ToString;

    #[test]
    fn core_error_converts() {
        let e = CityGmlError::from(ParseError::eof(0));
        assert!(matches!(e, CityGmlError::Core(_)));
    }

    #[test]
    fn display_variants() {
        assert_eq!(
            CityGmlError::xml_error(42, "malformed tag").to_string(),
            "XML error at offset 42: malformed tag"
        );
        assert_eq!(
            CityGmlError::invalid_lod(9, 7).to_string(),
            "invalid LOD level 9 at offset 7 (expected 0–4)"
        );
        assert_eq!(
            CityGmlError::missing_element("cityObjectMember", 10).to_string(),
            "missing required element \"cityObjectMember\" at offset 10"
        );
        assert_eq!(
            CityGmlError::Unsupported("appearance").to_string(),
            "CityGML unsupported feature: appearance"
        );
    }
}
