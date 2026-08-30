//! Error type for the IFC parser, built on [`tpt_io_core::ParseError`].

#[cfg(not(any(feature = "alloc", feature = "std")))]
compile_error!(
    "tpt-io-ifc requires the `alloc` or `std` feature: its error type owns structured parse data."
);

use alloc::string::String;
use tpt_io_core::ParseError;

/// Errors produced while parsing IFC (STEP-Physical File) data.
///
/// The `Core` variant wraps a [`tpt_io_core::ParseError`] carrying the byte
/// offset, expected-vs-actual token, spec reference, and surrounding bytes.
/// The remaining variants cover IFC/SPF-specific structural failures that
/// arise during tokenization, entity resolution, and relationship traversal.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum IfcError {
    /// A structural error reported by the shared core.
    Core(ParseError),
    /// A IFC-specific feature or construct that is not yet supported.
    Unsupported(&'static str),
    /// The SPF tokenizer encountered an invalid byte or malformed token.
    InvalidToken {
        /// Byte offset where the token began.
        offset: usize,
        /// The offending token text.
        token: String,
    },
    /// A string literal was not properly terminated before end-of-input.
    UnterminatedString {
        /// Byte offset where the string began.
        offset: usize,
    },
    /// A numeric literal (integer or real) could not be parsed.
    InvalidNumber {
        /// Byte offset of the numeric token.
        offset: usize,
        /// The offending token text.
        token: String,
    },
    /// An entity instance identifier (e.g. `#123`) was malformed.
    InvalidInstanceId {
        /// Byte offset of the malformed id.
        offset: usize,
        /// The offending token text.
        token: String,
    },
    /// A reference (`#id`) points to a non-existent entity instance.
    UnknownInstance {
        /// Byte offset of the dangling reference.
        offset: usize,
        /// The target instance id that was not found.
        id: usize,
    },
    /// A required section (HEADER or DATA) is missing or appears out of order.
    MissingSection(&'static str),
    /// The file does not begin with the mandated `ISO-10303-21;` header.
    InvalidHeader,
    /// A required argument of an entity instance was missing.
    MissingArgument {
        /// Byte offset of the entity instance.
        offset: usize,
        /// The entity type name.
        entity: String,
        /// The zero-based index of the missing argument.
        index: usize,
    },
    /// An argument type did not match the schema expectation.
    InvalidArgument {
        /// Byte offset of the offending argument.
        offset: usize,
        /// Human-readable description of the type mismatch.
        detail: String,
    },
    /// The entity type is not part of the detected IFC schema version.
    UnknownEntityType {
        /// Byte offset of the entity instance.
        offset: usize,
        /// The unrecognized type name.
        entity: String,
    },
    /// A relationship traversal (spatial, material, property) failed.
    RelationshipError {
        /// Description of the traversal failure.
        detail: String,
    },
    /// Any other format-specific failure with a human-readable reason.
    Custom(String),
}

impl IfcError {
    /// Build an `InvalidToken` error.
    pub fn invalid_token(offset: usize, token: impl Into<String>) -> Self {
        IfcError::InvalidToken { offset, token: token.into() }
    }

    /// Build an `UnterminatedString` error.
    pub fn unterminated_string(offset: usize) -> Self {
        IfcError::UnterminatedString { offset }
    }

    /// Build an `InvalidNumber` error.
    pub fn invalid_number(offset: usize, token: impl Into<String>) -> Self {
        IfcError::InvalidNumber { offset, token: token.into() }
    }

    /// Build an `InvalidInstanceId` error.
    pub fn invalid_instance_id(offset: usize, token: impl Into<String>) -> Self {
        IfcError::InvalidInstanceId { offset, token: token.into() }
    }

    /// Build an `UnknownInstance` error.
    pub fn unknown_instance(offset: usize, id: usize) -> Self {
        IfcError::UnknownInstance { offset, id }
    }

    /// Build a `MissingArgument` error.
    pub fn missing_argument(offset: usize, entity: impl Into<String>, index: usize) -> Self {
        IfcError::MissingArgument { offset, entity: entity.into(), index }
    }

    /// Build an `InvalidArgument` error.
    pub fn invalid_argument(offset: usize, detail: impl Into<String>) -> Self {
        IfcError::InvalidArgument { offset, detail: detail.into() }
    }

    /// Build an `UnknownEntityType` error.
    pub fn unknown_entity_type(offset: usize, entity: impl Into<String>) -> Self {
        IfcError::UnknownEntityType { offset, entity: entity.into() }
    }

    /// Build a `RelationshipError`.
    pub fn relationship(detail: impl Into<String>) -> Self {
        IfcError::RelationshipError { detail: detail.into() }
    }

    /// Build a generic `Custom` error.
    pub fn custom(msg: impl Into<String>) -> Self {
        IfcError::Custom(msg.into())
    }
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
            IfcError::InvalidToken { offset, token } => {
                write!(f, "invalid token {token:?} at offset {offset}")
            }
            IfcError::UnterminatedString { offset } => {
                write!(f, "unterminated string starting at offset {offset}")
            }
            IfcError::InvalidNumber { offset, token } => {
                write!(f, "invalid number {token:?} at offset {offset}")
            }
            IfcError::InvalidInstanceId { offset, token } => {
                write!(f, "invalid instance id {token:?} at offset {offset}")
            }
            IfcError::UnknownInstance { offset, id } => {
                write!(f, "reference to unknown instance #{id} at offset {offset}")
            }
            IfcError::MissingSection(section) => write!(f, "missing required section: {section}"),
            IfcError::InvalidHeader => write!(f, "file does not begin with ISO-10303-21"),
            IfcError::MissingArgument { offset, entity, index } => {
                write!(f, "missing argument {index} of {entity:?} at offset {offset}")
            }
            IfcError::InvalidArgument { offset, detail } => {
                write!(f, "invalid argument at offset {offset}: {detail}")
            }
            IfcError::UnknownEntityType { offset, entity } => {
                write!(f, "unknown entity type {entity:?} at offset {offset}")
            }
            IfcError::RelationshipError { detail } => {
                write!(f, "relationship traversal error: {detail}")
            }
            IfcError::Custom(msg) => write!(f, "{msg}"),
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

    #[test]
    fn display_variants() {
        assert_eq!(
            IfcError::invalid_token(5, "�").to_string(),
            "invalid token \"\u{FFFD}\" at offset 5"
        );
        assert_eq!(
            IfcError::unknown_instance(42, 99).to_string(),
            "reference to unknown instance #99 at offset 42"
        );
        assert_eq!(
            IfcError::MissingSection("DATA").to_string(),
            "missing required section: DATA"
        );
        assert_eq!(
            IfcError::Unsupported("IFC4X3 torsion").to_string(),
            "IFC unsupported feature: IFC4X3 torsion"
        );
    }
}
