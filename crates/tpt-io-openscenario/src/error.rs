//! Error type for the OpenSCENARIO reader, built on [`tpt_io_core::ParseError`].

#[cfg(not(any(feature = "alloc", feature = "std")))]
compile_error!(
    "tpt-io-openscenario requires the `alloc` or `std` feature: its error type owns text."
);

use alloc::string::String;

use tpt_io_core::ParseError;

/// Errors produced while reading or validating an OpenSCENARIO file.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum OpenScenarioError {
    /// Malformed XML or a malformed attribute, with its byte offset.
    Core(ParseError),
    /// The scenario parsed but breaks a rule.
    Invalid(String),
}

impl From<ParseError> for OpenScenarioError {
    fn from(e: ParseError) -> Self {
        OpenScenarioError::Core(e)
    }
}

impl core::fmt::Display for OpenScenarioError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            OpenScenarioError::Core(e) => write!(f, "{e}"),
            OpenScenarioError::Invalid(what) => write!(f, "invalid OpenSCENARIO file: {what}"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for OpenScenarioError {}
