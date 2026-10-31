//! Error type for the MTConnect reader, built on [`tpt_io_core::ParseError`].

#[cfg(not(any(feature = "alloc", feature = "std")))]
compile_error!("tpt-io-mtconnect requires the `alloc` or `std` feature: its error type owns text.");

use alloc::string::String;

use tpt_io_core::ParseError;

/// Errors produced while reading or cross-checking MTConnect documents.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum MtConnectError {
    /// Malformed XML or a missing required attribute, with its byte offset.
    Core(ParseError),
    /// An observation refers to a data item that the devices document does not define.
    UnknownDataItem(String),
    /// The document is not the kind this reader expects.
    Invalid(String),
}

impl From<ParseError> for MtConnectError {
    fn from(e: ParseError) -> Self {
        MtConnectError::Core(e)
    }
}

impl core::fmt::Display for MtConnectError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            MtConnectError::Core(e) => write!(f, "{e}"),
            MtConnectError::UnknownDataItem(id) => write!(f, "observation refers to unknown data item '{id}'"),
            MtConnectError::Invalid(what) => write!(f, "invalid MTConnect document: {what}"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for MtConnectError {}
