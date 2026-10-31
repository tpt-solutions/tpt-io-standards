//! Error type for the HL7 v2 reader, built on [`tpt_io_core::ParseError`].

#[cfg(not(any(feature = "alloc", feature = "std")))]
compile_error!("tpt-io-hl7v2 requires the `alloc` or `std` feature: its error type owns text.");

use alloc::string::String;

use tpt_io_core::ParseError;

/// Errors produced while reading an HL7 v2 message.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Hl7Error {
    /// The bytes are not text, with the byte offset of the problem.
    Core(ParseError),
    /// The message breaks the rules HL7 v2 requires (for example, no leading `MSH`).
    Invalid(String),
}

impl From<ParseError> for Hl7Error {
    fn from(e: ParseError) -> Self {
        Hl7Error::Core(e)
    }
}

impl core::fmt::Display for Hl7Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Hl7Error::Core(e) => write!(f, "{e}"),
            Hl7Error::Invalid(what) => write!(f, "invalid HL7 v2 message: {what}"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for Hl7Error {}
