//! Error type for the IPC-2581 reader, built on [`tpt_io_core::ParseError`].

#[cfg(not(any(feature = "alloc", feature = "std")))]
compile_error!("tpt-io-ipc2581 requires the `alloc` or `std` feature: its error type owns text.");

use alloc::string::String;

use tpt_io_core::ParseError;

/// Errors produced while reading an IPC-2581 file.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Ipc2581Error {
    /// Malformed XML, with its byte offset.
    Core(ParseError),
    /// The root is not `IPC-2581` or a required attribute is missing.
    Invalid(String),
}

impl From<ParseError> for Ipc2581Error {
    fn from(e: ParseError) -> Self {
        Ipc2581Error::Core(e)
    }
}

impl core::fmt::Display for Ipc2581Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Ipc2581Error::Core(e) => write!(f, "{e}"),
            Ipc2581Error::Invalid(what) => write!(f, "invalid IPC-2581 file: {what}"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for Ipc2581Error {}
