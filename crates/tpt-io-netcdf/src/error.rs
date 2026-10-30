//! Error type for the NetCDF parser, built on [`tpt_io_core::ParseError`].

#[cfg(not(any(feature = "alloc", feature = "std")))]
compile_error!("tpt-io-netcdf requires the `alloc` or `std` feature: its error type owns structured parse data.");

use tpt_io_core::ParseError;

/// Errors produced while parsing NetCDF data.
///
/// The `Core` variant wraps a [`tpt_io_core::ParseError`] carrying the byte
/// offset, expected-vs-actual token, spec reference, and surrounding bytes.
/// Format-specific failure modes are added as additional variants as parsers
/// land (see `todo.md`).
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum NetCdfError {
    /// A structural error reported by the shared core.
    Core(ParseError),
    /// A NetCDF-specific feature or construct that is not yet supported.
    Unsupported(&'static str),
}

impl From<ParseError> for NetCdfError {
    fn from(e: ParseError) -> Self {
        NetCdfError::Core(e)
    }
}

impl core::fmt::Display for NetCdfError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            NetCdfError::Core(e) => write!(f, "{e}"),
            NetCdfError::Unsupported(feat) => write!(f, "NetCDF unsupported feature: {feat}"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for NetCdfError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn core_error_converts() {
        let e = NetCdfError::from(ParseError::eof(0));
        assert!(matches!(e, NetCdfError::Core(_)));
    }
}

impl From<tpt_io_hdf5::Hdf5Error> for NetCdfError {
    fn from(e: tpt_io_hdf5::Hdf5Error) -> Self {
        match e {
            tpt_io_hdf5::Hdf5Error::Core(c) => NetCdfError::Core(c),
            tpt_io_hdf5::Hdf5Error::Unsupported(s) => NetCdfError::Unsupported(s),
            _ => NetCdfError::Unsupported("unrecognised HDF5 error"),
        }
    }
}
