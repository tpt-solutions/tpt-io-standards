//! NetCDF-4 (HDF5-based) detection and superblock parsing.
//!
//! A NetCDF-4 file *is* an HDF5 file: its first 8 bytes are the HDF5 signature
//! `\x89HDF\r\n\x1a\n`. Full NetCDF-4 decoding (descending the HDF5 object
//! tree to datasets, reading chunked/deflate/shuffle-filtered data, and mapping
//! NetCDF conventions such as `_Netcdf4Dimid` and `_Netcdf4Coordinates`) is not
//! yet implemented — this module provides format detection plus enough
//! superblock metadata to identify and triage such files without panicking.

use crate::error::NetCdfError;

/// The high-level kind of a NetCDF file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileKind {
    /// NetCDF-3 classic / 64-bit offset (magic `CDF`).
    Classic,
    /// NetCDF-4 (HDF5 container).
    NetCdf4,
}

/// Identify the kind of a NetCDF byte stream, if recognized.
pub fn detect_format(data: &[u8]) -> Option<FileKind> {
    if data.len() >= 3 && &data[0..3] == b"CDF" {
        Some(FileKind::Classic)
    } else if data.len() >= 8 && &data[0..8] == b"\x89HDF\r\n\x1a\n" {
        Some(FileKind::NetCdf4)
    } else {
        None
    }
}

/// Parsed HDF5 superblock metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hdf5Superblock {
    /// Superblock version (1, 2, or 3).
    pub version: u8,
    /// Size in bytes of an HDF5 offset (`sizeof_off_t`).
    pub offset_size: u8,
    /// Size in bytes of an HDF5 length (`sizeof_size_t`).
    pub length_size: u8,
    /// File offset of the root group's object header / symbol table.
    pub root_group_addr: u64,
}

fn read_int(data: &[u8], pos: usize, size: usize) -> core::result::Result<u64, NetCdfError> {
    let s = data
        .get(pos..pos + size)
        .ok_or_else(|| NetCdfError::Core(tpt_io_core::ParseError::eof(pos)))?;
    let mut v = 0u64;
    for &b in s {
        v = (v << 8) | b as u64;
    }
    Ok(v)
}

/// Parse the HDF5 superblock of a NetCDF-4 file.
///
/// Returns [`NetCdfError::Unsupported`] for unrecognized superblock versions so
/// callers can surface a clear message rather than guessing.
pub fn parse_hdf5_superblock(data: &[u8]) -> core::result::Result<Hdf5Superblock, NetCdfError> {
    const SIG: &[u8; 8] = b"\x89HDF\r\n\x1a\n";
    if data.len() < 8 || &data[0..8] != SIG {
        return Err(NetCdfError::Unsupported("not an HDF5/NetCDF-4 file"));
    }
    let version = *data.get(8).ok_or_else(|| NetCdfError::Core(tpt_io_core::ParseError::eof(8)))?;
    let (offset_size, length_size, root_group_addr) = match version {
        // HDF5 superblock version 1: sizes live at bytes 13/14; the root group
        // symbol-table entry begins at offset 24 and leads with its offset.
        1 => {
            let os =
                *data.get(13).ok_or_else(|| NetCdfError::Core(tpt_io_core::ParseError::eof(13)))?;
            let ls =
                *data.get(14).ok_or_else(|| NetCdfError::Core(tpt_io_core::ParseError::eof(14)))?;
            let ra = read_int(data, 24, os as usize)?;
            (os, ls, ra)
        }
        // Superblock versions 2/3: offset/length sizes at 9/10, root group
        // address right after (offset 12).
        2 | 3 => {
            let os =
                *data.get(9).ok_or_else(|| NetCdfError::Core(tpt_io_core::ParseError::eof(9)))?;
            let ls =
                *data.get(10).ok_or_else(|| NetCdfError::Core(tpt_io_core::ParseError::eof(10)))?;
            let ra = read_int(data, 12, os as usize)?;
            (os, ls, ra)
        }
        _other => {
            return Err(NetCdfError::Unsupported(
                "unsupported HDF5 superblock version (only 1/2/3 handled)",
            ));
        }
    };
    Ok(Hdf5Superblock { version, offset_size, length_size, root_group_addr })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_classic_and_hdf5() {
        assert_eq!(detect_format(b"CDF\x01rest"), Some(FileKind::Classic));
        assert_eq!(detect_format(b"\x89HDF\r\n\x1a\nrest"), Some(FileKind::NetCdf4));
        assert_eq!(detect_format(b"XXXX"), None);
    }

    #[test]
    fn parses_v2_superblock() {
        // Signature + version 2, offset size 8, length size 8, reserved, then
        // root group address = 200 encoded as 8 big-endian bytes.
        let mut buf = b"\x89HDF\r\n\x1a\n".to_vec();
        buf.push(2); // version
        buf.push(8); // offset size
        buf.push(8); // length size
        buf.push(0); // reserved
        buf.extend_from_slice(&200u64.to_be_bytes()); // root group address
        let sb = parse_hdf5_superblock(&buf).unwrap();
        assert_eq!(sb.version, 2);
        assert_eq!(sb.offset_size, 8);
        assert_eq!(sb.length_size, 8);
        assert_eq!(sb.root_group_addr, 200);
    }

    #[test]
    fn rejects_bad_magic() {
        assert!(parse_hdf5_superblock(b"not hdf5!").is_err());
    }
}
