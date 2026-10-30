//! Error type for the KiCad reader, built on [`tpt_io_core::ParseError`].

#[cfg(not(any(feature = "alloc", feature = "std")))]
compile_error!("tpt-io-kicad requires the `alloc` or `std` feature: its error type owns structured parse data.");

use tpt_io_core::ParseError;

/// Errors produced while reading KiCad files.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum KicadError {
    /// A tokenizer or structural error, with its byte offset.
    Core(ParseError),
    /// The file is well-formed S-expression text but not a KiCad document
    /// this crate reads (for example, a `.kicad_pro` project file).
    Unsupported(&'static str),
}

impl From<ParseError> for KicadError {
    fn from(e: ParseError) -> Self {
        KicadError::Core(e)
    }
}

impl core::fmt::Display for KicadError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            KicadError::Core(e) => write!(f, "{e}"),
            KicadError::Unsupported(what) => write!(f, "unsupported KiCad document: {what}"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for KicadError {}
