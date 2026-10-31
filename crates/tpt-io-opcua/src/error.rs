//! Error type for the OPC UA NodeSet reader, built on [`tpt_io_core::ParseError`].

#[cfg(not(any(feature = "alloc", feature = "std")))]
compile_error!("tpt-io-opcua requires the `alloc` or `std` feature: its error type owns text.");

use alloc::string::String;

use tpt_io_core::ParseError;

/// Errors produced while reading or validating a NodeSet.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum OpcUaError {
    /// Malformed XML or a missing required attribute, with its byte offset.
    Core(ParseError),
    /// The NodeSet parsed but breaks a rule (for example, a duplicate NodeId).
    Invalid(String),
}

impl From<ParseError> for OpcUaError {
    fn from(e: ParseError) -> Self {
        OpcUaError::Core(e)
    }
}

impl core::fmt::Display for OpcUaError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            OpcUaError::Core(e) => write!(f, "{e}"),
            OpcUaError::Invalid(what) => write!(f, "invalid OPC UA NodeSet: {what}"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for OpcUaError {}
