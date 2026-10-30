//! Error type for the HDF5 reader, built on [`tpt_io_core::ParseError`].

#[cfg(not(any(feature = "alloc", feature = "std")))]
compile_error!(
    "tpt-io-hdf5 requires the `alloc` or `std` feature: its error type owns structured parse data."
);

use tpt_io_core::ParseError;

/// Errors produced while reading HDF5 data.
///
/// `Core` wraps a [`tpt_io_core::ParseError`] carrying the byte offset and
/// surrounding bytes. `Unsupported` names an HDF5 construct the reader does
/// not handle yet, so callers can distinguish "malformed" from "not supported".
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Hdf5Error {
    /// A structural error reported by the shared core.
    Core(ParseError),
    /// An HDF5 feature or construct that is not yet supported.
    Unsupported(&'static str),
}

impl From<ParseError> for Hdf5Error {
    fn from(e: ParseError) -> Self {
        Hdf5Error::Core(e)
    }
}

impl core::fmt::Display for Hdf5Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Hdf5Error::Core(e) => write!(f, "{e}"),
            Hdf5Error::Unsupported(feat) => write!(f, "HDF5 unsupported feature: {feat}"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for Hdf5Error {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn core_error_converts() {
        let e = Hdf5Error::from(ParseError::eof(0));
        assert!(matches!(e, Hdf5Error::Core(_)));
    }
}
