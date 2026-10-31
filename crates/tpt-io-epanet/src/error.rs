//! Error type for the EPANET reader, built on [`tpt_io_core::ParseError`].

#[cfg(not(any(feature = "alloc", feature = "std")))]
compile_error!("tpt-io-epanet requires the `alloc` or `std` feature: its error type owns text.");

use alloc::string::String;

use tpt_io_core::ParseError;

/// Errors produced while reading, writing or validating an EPANET network.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum EpanetError {
    /// A row outside any section, or an unterminated section header, with its byte offset.
    Core(ParseError),
    /// A row in a typed section that does not have the fields it needs.
    Invalid(String),
    /// The network parsed but breaks a rule (for example, a pipe to an unknown node).
    Topology(String),
}

impl From<ParseError> for EpanetError {
    fn from(e: ParseError) -> Self {
        EpanetError::Core(e)
    }
}

impl core::fmt::Display for EpanetError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            EpanetError::Core(e) => write!(f, "{e}"),
            EpanetError::Invalid(what) => write!(f, "invalid EPANET row: {what}"),
            EpanetError::Topology(what) => write!(f, "invalid EPANET network: {what}"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for EpanetError {}
