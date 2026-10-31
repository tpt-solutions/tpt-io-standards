//! Error type for the Verilog reader, built on [`tpt_io_core::ParseError`].

#[cfg(not(any(feature = "alloc", feature = "std")))]
compile_error!("tpt-io-verilog requires the `alloc` or `std` feature: its error type owns text.");

use alloc::string::String;

use tpt_io_core::ParseError;

/// Errors produced while reading a Verilog netlist.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum VerilogError {
    /// A token the grammar does not allow here, with its byte offset.
    Core(ParseError),
    /// A construct outside the structural subset (for example `always`).
    Unsupported(String),
}

impl From<ParseError> for VerilogError {
    fn from(e: ParseError) -> Self {
        VerilogError::Core(e)
    }
}

impl core::fmt::Display for VerilogError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            VerilogError::Core(e) => write!(f, "{e}"),
            VerilogError::Unsupported(what) => write!(f, "unsupported Verilog construct: {what}"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for VerilogError {}
