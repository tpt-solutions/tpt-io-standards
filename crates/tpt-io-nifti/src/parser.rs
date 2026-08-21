//! NIfTI reader, validator, and high-level parse entry points.
use tpt_io_core::{ParseError, Reader, Validator};

use crate::error::NiftiError;
use crate::header::parse_header;
use crate::model::{from_parts, NiftiFile};

/// A NIfTI reader implementing [`tpt_io_core::Reader`].
#[derive(Debug, Clone, Default)]
pub struct NiftiReader;

impl NiftiReader {
    /// Create a new reader.
    pub fn new() -> Self {
        Self
    }
}

impl Reader<NiftiFile> for NiftiReader {
    type Error = NiftiError;

    fn parse(&self, input: &[u8]) -> core::result::Result<NiftiFile, NiftiError> {
        parse_nii(input)
    }
}

/// Parse a single-file `.nii` (after optional gzip decompression by the caller).
pub fn parse_nii(data: &[u8]) -> core::result::Result<NiftiFile, NiftiError> {
    let header = parse_header(data)?;
    let start = header.vox_offset as usize;
    let end = start + header.data_bytes();
    let data_block =
        data.get(start..end).ok_or_else(|| NiftiError::Core(ParseError::eof(data.len())))?.to_vec();
    from_parts(header, data_block)
}

/// Parse the dual-file `.hdr`/`.img` representation from separate byte blocks.
pub fn parse_hdr_img(hdr: &[u8], img: &[u8]) -> core::result::Result<NiftiFile, NiftiError> {
    let header = parse_header(hdr)?;
    let start = header.vox_offset as usize;
    let end = start + header.data_bytes();
    let data_block =
        img.get(start..end).ok_or_else(|| NiftiError::Core(ParseError::eof(img.len())))?.to_vec();
    from_parts(header, data_block)
}

/// Parse a gzip-compressed `.nii.gz` stream (requires the `gzip` feature).
#[cfg(feature = "gzip")]
pub fn parse_nii_gz(data: &[u8]) -> core::result::Result<NiftiFile, NiftiError> {
    use flate2::read::GzDecoder;
    use std::io::Read;
    let mut dec = GzDecoder::new(data);
    let mut out = Vec::new();
    dec.read_to_end(&mut out).map_err(|_| NiftiError::Unsupported("gzip decode failed"))?;
    parse_nii(&out)
}

/// NIfTI syntactic/semantic validator.
#[derive(Debug, Clone, Default)]
pub struct NiftiValidator;
impl NiftiValidator {
    /// Create a new validator.
    pub fn new() -> Self {
        Self
    }
}

impl Validator<NiftiFile> for NiftiValidator {
    type Error = NiftiError;

    fn validate_syntactic(&self, value: &NiftiFile) -> core::result::Result<(), NiftiError> {
        if value.header.ndim < 1 || value.header.ndim > 7 {
            return Err(NiftiError::Core(ParseError::custom(
                0,
                "dimension count out of range (1..7)",
            )));
        }
        if value.header.datatype.size_of() == 0 {
            return Err(NiftiError::Unsupported("unknown datatype"));
        }
        Ok(())
    }

    fn validate_semantic(&self, value: &NiftiFile) -> core::result::Result<(), NiftiError> {
        if value.data.len() != value.header.data_bytes() {
            return Err(NiftiError::Core(ParseError::custom(0, "voxel data length mismatch")));
        }
        Ok(())
    }
}
