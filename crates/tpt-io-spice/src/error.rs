//! Error type for the SPICE parser, built on [`tpt_io_core::ParseError`].

#[cfg(not(any(feature = "alloc", feature = "std")))]
compile_error!("tpt-io-spice requires the `alloc` or `std` feature: its error type owns structured parse data.");

use alloc::string::String;
use tpt_io_core::ParseError;

/// Errors produced while parsing SPICE netlist data.
///
/// The `Core` variant wraps a [`tpt_io_core::ParseError`] carrying the byte
/// offset, expected-vs-actual token, spec reference, and surrounding bytes.
/// The remaining variants cover SPICE-specific structural failures.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum SpiceError {
    /// A structural error reported by the shared core.
    Core(ParseError),
    /// A value token (e.g. a component value or parameter) could not be parsed.
    InvalidValue {
        /// The offending token.
        token: String,
    },
    /// An unrecognized element designator (first letter of an instance).
    UnknownElement(char),
    /// A required field (e.g. a node) was missing from a card.
    MissingField(&'static str),
    /// A SPICE-specific feature or construct that is not yet supported.
    Unsupported(&'static str),
}

impl From<ParseError> for SpiceError {
    fn from(e: ParseError) -> Self {
        SpiceError::Core(e)
    }
}

impl core::fmt::Display for SpiceError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            SpiceError::Core(e) => write!(f, "{e}"),
            SpiceError::InvalidValue { token } => write!(f, "invalid value token {token:?}"),
            SpiceError::UnknownElement(c) => write!(f, "unknown element designator {c:?}"),
            SpiceError::MissingField(field) => write!(f, "missing required field {field}"),
            SpiceError::Unsupported(feat) => write!(f, "SPICE unsupported feature: {feat}"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for SpiceError {}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(not(feature = "std"))]
    use alloc::string::ToString;

    #[test]
    fn core_error_converts() {
        let e = SpiceError::from(ParseError::eof(0));
        assert!(matches!(e, SpiceError::Core(_)));
    }

    #[test]
    fn display_invalid_value() {
        let e = SpiceError::InvalidValue { token: "abc".into() };
        assert_eq!(e.to_string(), "invalid value token \"abc\"");
    }
}
