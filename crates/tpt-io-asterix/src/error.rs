//! Error type for the ASTERIX reader, built on [`tpt_io_core::ParseError`].

#[cfg(not(any(feature = "alloc", feature = "std")))]
compile_error!("tpt-io-asterix requires the `alloc` or `std` feature: its error type owns text.");

use tpt_io_core::ParseError;

/// Errors produced while reading ASTERIX data.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum AsterixError {
    /// A block whose length is inconsistent with its contents, with its byte offset.
    Core(ParseError),
    /// A record uses a data item this crate does not know the length of.
    UnsupportedItem { category: u8, frn: usize },
}

impl From<ParseError> for AsterixError {
    fn from(e: ParseError) -> Self {
        AsterixError::Core(e)
    }
}

impl core::fmt::Display for AsterixError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            AsterixError::Core(e) => write!(f, "{e}"),
            AsterixError::UnsupportedItem { category, frn } => {
                write!(f, "category {category:03} item FRN {frn} is not supported")
            }
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for AsterixError {}
