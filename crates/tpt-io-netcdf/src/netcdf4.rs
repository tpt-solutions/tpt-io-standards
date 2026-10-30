//! NetCDF-4 entry points.
//!
//! A NetCDF-4 file is an HDF5 file, so the reader lives in `tpt-io-hdf5`. This
//! module re-exports it under the NetCDF crate's paths and adds the NetCDF-level
//! format detection that tells classic NetCDF-3 files from NetCDF-4 files.

pub use tpt_io_hdf5::{
    parse, parse_hdf5_superblock, Dataset, Datatype, DecodedData, Filter, Group, H5Attr, Hdf5File,
    Hdf5Superblock, Layout, Link,
};

/// The high-level kind of a NetCDF file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileKind {
    Classic,
    NetCdf4,
}

/// Detect a classic NetCDF-3 file (`CDF` magic) or a NetCDF-4 file (HDF5 signature).
pub fn detect_format(data: &[u8]) -> Option<FileKind> {
    if data.len() >= 3 && &data[0..3] == b"CDF" {
        Some(FileKind::Classic)
    } else if tpt_io_hdf5::is_hdf5(data) {
        Some(FileKind::NetCdf4)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_classic_and_netcdf4() {
        assert_eq!(detect_format(b"CDF\x01rest"), Some(FileKind::Classic));
        assert_eq!(
            detect_format(&[0x89, 0x48, 0x44, 0x46, 0x0d, 0x0a, 0x1a, 0x0a]),
            Some(FileKind::NetCdf4)
        );
        assert_eq!(detect_format(b"nope"), None);
    }
}
