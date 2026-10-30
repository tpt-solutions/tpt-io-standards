//! Error type for the Gerber parser, built on [`tpt_io_core::ParseError`].

#[cfg(not(any(feature = "alloc", feature = "std")))]
compile_error!("tpt-io-gerber requires the `alloc` or `std` feature: its error type owns structured parse data.");

use alloc::string::String;
use tpt_io_core::ParseError;

/// Errors produced while parsing Gerber (RS-274X / X2 / X3) or Excellon data.
///
/// The `Core` variant wraps a [`tpt_io_core::ParseError`] carrying the byte
/// offset, expected-vs-actual token, spec reference, and surrounding bytes.
/// The remaining variants cover Gerber/Excellon-specific structural failures.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum GerberError {
    /// A structural error reported by the shared core.
    Core(ParseError),
    /// A Gerber-specific feature or construct that is not yet supported.
    Unsupported(&'static str),
    /// An unrecognized or malformed command at `offset`.
    InvalidCommand {
        /// Byte offset where the command began.
        offset: usize,
        /// The offending command text.
        command: String,
    },
    /// A value to be written does not fit the file's coordinate format.
    CoordinateOutOfRange {
        /// The axis, `'X'` or `'Y'`.
        axis: char,
        /// The offending value, as written.
        token: String,
    },
    /// A coordinate token could not be interpreted as an integer.
    InvalidCoordinate {
        /// Byte offset of the coordinate token.
        offset: usize,
        /// The offending token text.
        token: String,
    },
    /// An aperture definition (`%ADD...%`) could not be parsed.
    InvalidAperture {
        /// Byte offset of the aperture block.
        offset: usize,
        /// The offending block text.
        token: String,
    },
    /// A numeric field (e.g. a modifier) could not be parsed as a number.
    InvalidNumber {
        /// Byte offset of the value.
        offset: usize,
        /// The offending value text.
        value: String,
    },
    /// A required field (e.g. a coordinate or modifier) was missing.
    MissingField(&'static str),
    /// The input ended before the parser expected more data.
    UnexpectedEof,
    /// A round-trip serialization produced an internally inconsistent value.
    RoundTrip(String),
    /// Any other format-specific failure with a human-readable reason.
    Custom(String),
}

impl GerberError {
    /// Build an `InvalidCommand` error.
    pub fn invalid_command(offset: usize, command: impl Into<String>) -> Self {
        GerberError::InvalidCommand { offset, command: command.into() }
    }

    /// Build an `InvalidCoordinate` error.
    pub fn invalid_coordinate(offset: usize, token: impl Into<String>) -> Self {
        GerberError::InvalidCoordinate { offset, token: token.into() }
    }

    /// Build an `InvalidAperture` error.
    pub fn invalid_aperture(offset: usize, token: impl Into<String>) -> Self {
        GerberError::InvalidAperture { offset, token: token.into() }
    }

    /// Build an `InvalidNumber` error.
    pub fn invalid_number(offset: usize, value: impl Into<String>) -> Self {
        GerberError::InvalidNumber { offset, value: value.into() }
    }

    /// Build a generic `RoundTrip` error.
    pub fn round_trip(msg: impl Into<String>) -> Self {
        GerberError::RoundTrip(msg.into())
    }

    /// Build a generic `Custom` error.
    pub fn custom(msg: impl Into<String>) -> Self {
        GerberError::Custom(msg.into())
    }
}

impl From<ParseError> for GerberError {
    fn from(e: ParseError) -> Self {
        GerberError::Core(e)
    }
}

impl core::fmt::Display for GerberError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            GerberError::Core(e) => write!(f, "{e}"),
            GerberError::Unsupported(feat) => write!(f, "Gerber unsupported feature: {feat}"),
            GerberError::InvalidCommand { offset, command } => {
                write!(f, "invalid command {command:?} at offset {offset}")
            }
            GerberError::CoordinateOutOfRange { axis, token } => {
                write!(f, "{axis} coordinate {token} does not fit the file's coordinate format")
            }
            GerberError::InvalidCoordinate { offset, token } => {
                write!(f, "invalid coordinate {token:?} at offset {offset}")
            }
            GerberError::InvalidAperture { offset, token } => {
                write!(f, "invalid aperture definition {token:?} at offset {offset}")
            }
            GerberError::InvalidNumber { offset, value } => {
                write!(f, "invalid number {value:?} at offset {offset}")
            }
            GerberError::MissingField(field) => write!(f, "missing required field {field}"),
            GerberError::UnexpectedEof => write!(f, "unexpected end of input"),
            GerberError::RoundTrip(msg) => write!(f, "round-trip error: {msg}"),
            GerberError::Custom(msg) => write!(f, "{msg}"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for GerberError {}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(not(feature = "std"))]
    use alloc::string::ToString;

    #[test]
    fn core_error_converts() {
        let e = GerberError::from(ParseError::eof(0));
        assert!(matches!(e, GerberError::Core(_)));
    }

    #[test]
    fn display_variants() {
        assert_eq!(
            GerberError::invalid_command(3, "G99").to_string(),
            "invalid command \"G99\" at offset 3"
        );
        assert_eq!(
            GerberError::MissingField("X coordinate").to_string(),
            "missing required field X coordinate"
        );
        assert_eq!(
            GerberError::Unsupported("step-repeat").to_string(),
            "Gerber unsupported feature: step-repeat"
        );
    }
}
