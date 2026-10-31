//! Error type for the GLM reader, built on [`tpt_io_core::ParseError`].

#[cfg(not(any(feature = "alloc", feature = "std")))]
compile_error!("tpt-io-gridlabd requires the `alloc` or `std` feature: its error type owns text.");

use alloc::string::String;

use tpt_io_core::ParseError;

/// Errors produced while reading, writing or validating a GLM model.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum GlmError {
    /// A syntax error, with its byte offset.
    Core(ParseError),
    /// The model parsed but breaks a naming rule.
    Invalid(String),
}

impl From<ParseError> for GlmError {
    fn from(e: ParseError) -> Self {
        GlmError::Core(e)
    }
}

impl core::fmt::Display for GlmError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            GlmError::Core(e) => write!(f, "{e}"),
            GlmError::Invalid(what) => write!(f, "invalid GLM model: {what}"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for GlmError {}
