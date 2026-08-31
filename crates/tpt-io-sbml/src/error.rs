//! Error type for the SBML parser, built on [`tpt_io_core::ParseError`].

#[cfg(not(any(feature = "alloc", feature = "std")))]
compile_error!(
    "tpt-io-sbml requires the `alloc` or `std` feature: its error type owns structured parse data."
);

use alloc::string::{String, ToString};

use tpt_io_core::ParseError;

/// Errors produced while parsing SBML data.
///
/// The `Core` variant wraps a [`tpt_io_core::ParseError`] carrying the byte
/// offset, expected-vs-actual token, spec reference, and surrounding bytes.
/// Format-specific failure modes are layered on top as additional variants.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum SbmlError {
    /// A structural error reported by the shared core.
    Core(ParseError),
    /// An error surfaced by the underlying XML reader (e.g. malformed markup).
    Xml(String),
    /// A SBML-specific feature or construct that is not yet supported.
    Unsupported(&'static str),
    /// A required attribute was missing from an element.
    MissingAttribute {
        /// Element local name.
        element: String,
        /// Missing attribute name.
        attr: String,
    },
    /// A required child element was missing from a parent.
    MissingElement {
        /// Parent element local name.
        parent: &'static str,
        /// Missing child element local name.
        child: &'static str,
    },
    /// An attribute value could not be interpreted as the expected type.
    InvalidValue {
        /// Element local name.
        element: String,
        /// Attribute name.
        attr: String,
        /// The offending value.
        value: String,
    },
    /// A duplicate identifier was encountered (SBML requires SId uniqueness
    /// within the relevant scope).
    DuplicateId(String),
    /// A `ci` reference in a MathML expression resolved to an unknown identifier.
    UnknownReference(String),
    /// MathML could not be parsed into an expression tree.
    MathParse(String),
    /// A semantic/consistency validation issue (mass/charge balance, missing
    /// references, duplicate or missing identifiers, …).
    Validation(String),
    /// An I/O error occurred while writing the document.
    Io(String),
}

impl From<ParseError> for SbmlError {
    fn from(e: ParseError) -> Self {
        SbmlError::Core(e)
    }
}

impl From<quick_xml::Error> for SbmlError {
    fn from(e: quick_xml::Error) -> Self {
        SbmlError::Xml(e.to_string())
    }
}

#[cfg(feature = "std")]
impl From<std::io::Error> for SbmlError {
    fn from(e: std::io::Error) -> Self {
        SbmlError::Io(e.to_string())
    }
}

impl SbmlError {
    /// Helper to build a [`SbmlError::InvalidValue`].
    pub(crate) fn invalid_value(element: &str, attr: &str, value: impl Into<String>) -> Self {
        SbmlError::InvalidValue {
            element: element.to_string(),
            attr: attr.to_string(),
            value: value.into(),
        }
    }

    /// Helper to build a [`SbmlError::MissingAttribute`].
    pub(crate) fn missing_attr(element: &str, attr: &str) -> Self {
        SbmlError::MissingAttribute { element: element.to_string(), attr: attr.to_string() }
    }
}

impl core::fmt::Display for SbmlError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            SbmlError::Core(e) => write!(f, "{e}"),
            SbmlError::Xml(s) => write!(f, "XML error: {s}"),
            SbmlError::Unsupported(feat) => write!(f, "SBML unsupported feature: {feat}"),
            SbmlError::MissingAttribute { element, attr } => {
                write!(f, "<{element}> is missing required attribute '{attr}'")
            }
            SbmlError::MissingElement { parent, child } => {
                write!(f, "<{parent}> is missing required child element <{child}>")
            }
            SbmlError::InvalidValue { element, attr, value } => {
                write!(f, "invalid value for <{element}>@{attr}: '{value}'")
            }
            SbmlError::DuplicateId(id) => write!(f, "duplicate identifier '{id}'"),
            SbmlError::UnknownReference(id) => {
                write!(f, "MathML references unknown identifier '{id}'")
            }
            SbmlError::MathParse(s) => write!(f, "MathML parse error: {s}"),
            SbmlError::Validation(s) => write!(f, "validation error: {s}"),
            SbmlError::Io(s) => write!(f, "I/O error: {s}"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for SbmlError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn core_error_converts() {
        let e = SbmlError::from(ParseError::eof(0));
        assert!(matches!(e, SbmlError::Core(_)));
    }

    #[test]
    fn display_variants() {
        assert_eq!(
            SbmlError::missing_attr("species", "id").to_string(),
            "<species> is missing required attribute 'id'"
        );
        assert_eq!(SbmlError::DuplicateId("S1".into()).to_string(), "duplicate identifier 'S1'");
    }
}
