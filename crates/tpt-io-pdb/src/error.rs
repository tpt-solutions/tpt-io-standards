//! Error type for the PDB reader, built on [`tpt_io_core::ParseError`].

#[cfg(not(any(feature = "alloc", feature = "std")))]
compile_error!("tpt-io-pdb requires the `alloc` or `std` feature: its error type owns text.");

use alloc::string::String;

use tpt_io_core::ParseError;

/// Errors produced while reading a PDB file.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum PdbError {
    /// A record field that is not a number, or a record shorter than its fixed columns.
    Core(ParseError),
    /// The file parsed but has duplicate atom serial numbers.
    Invalid(String),
}

impl From<ParseError> for PdbError {
    fn from(e: ParseError) -> Self {
        PdbError::Core(e)
    }
}

impl core::fmt::Display for PdbError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            PdbError::Core(e) => write!(f, "{e}"),
            PdbError::Invalid(what) => write!(f, "invalid PDB file: {what}"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for PdbError {}
