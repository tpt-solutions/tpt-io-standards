//! Error type for the DXF reader, built on [`tpt_io_core::ParseError`].

#[cfg(not(any(feature = "alloc", feature = "std")))]
compile_error!("tpt-io-dxf requires the `alloc` or `std` feature: its error type owns text.");

use alloc::string::String;

use tpt_io_core::ParseError;

/// Errors produced while reading a DXF file.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum DxfError {
    /// A group code that is not an integer, or a value that is not a number, with its byte offset.
    Core(ParseError),
    /// A group-code stream that ends mid-pair or has an impossible structure.
    Invalid(String),
}

impl From<ParseError> for DxfError {
    fn from(e: ParseError) -> Self {
        DxfError::Core(e)
    }
}

impl core::fmt::Display for DxfError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            DxfError::Core(e) => write!(f, "{e}"),
            DxfError::Invalid(what) => write!(f, "invalid DXF: {what}"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for DxfError {}
