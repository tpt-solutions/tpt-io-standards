//! NIfTI header serialization (round-trip).
//!
//! [`serialize_header`] rebuilds the 348-byte NIfTI-1 header from the typed
//! [`Nifti1Header`]. The voxel data is stored separately and re-attached by the
//! caller, so a `parse → serialize → parse` cycle reproduces the typed header.

use alloc::vec;
use alloc::vec::Vec;

use crate::error::NiftiError;
use crate::header::{parse_header, Nifti1Header, MAGIC_NIFTI_1};

fn wr_i16(buf: &mut [u8], offset: usize, v: i16) {
    let b = v.to_le_bytes();
    buf[offset..offset + 2].copy_from_slice(&b);
}

fn wr_i32(buf: &mut [u8], offset: usize, v: i32) {
    let b = v.to_le_bytes();
    buf[offset..offset + 4].copy_from_slice(&b);
}

fn wr_f32(buf: &mut [u8], offset: usize, v: f32) {
    let b = v.to_le_bytes();
    buf[offset..offset + 4].copy_from_slice(&b);
}

fn wr_cstr(buf: &mut [u8], offset: usize, len: usize, s: &str) {
    let bytes = s.as_bytes();
    for (i, &b) in bytes.iter().take(len).enumerate() {
        buf[offset + i] = b;
    }
    if bytes.len() < len {
        buf[offset + bytes.len()] = 0;
    }
}

/// Serialize a typed header back to the 348-byte NIfTI-1 representation.
pub fn serialize_header(h: &Nifti1Header) -> Vec<u8> {
    let mut buf = vec![0u8; 348];
    wr_i32(&mut buf, 0, 348);
    wr_i16(&mut buf, 40, h.ndim as i16);
    for i in 0..7 {
        wr_i16(&mut buf, 42 + i * 2, h.dim[i] as i16);
        wr_f32(&mut buf, 76 + i * 4, h.pixdim[i]);
    }
    wr_i16(&mut buf, 70, h.datatype.code());
    wr_i16(&mut buf, 72, h.bitpix as i16);
    wr_f32(&mut buf, 108, h.vox_offset as f32);
    wr_f32(&mut buf, 112, h.scl_slope);
    wr_f32(&mut buf, 116, h.scl_inter);
    wr_i16(&mut buf, 252 - 2, h.qform_code);
    wr_i16(&mut buf, 252, h.sform_code);
    wr_f32(&mut buf, 256, h.quatern[0]);
    wr_f32(&mut buf, 260, h.quatern[1]);
    wr_f32(&mut buf, 264, h.quatern[2]);
    wr_f32(&mut buf, 268, h.qoffset[0]);
    wr_f32(&mut buf, 272, h.qoffset[1]);
    wr_f32(&mut buf, 276, h.qoffset[2]);
    for r in 0..3 {
        for c in 0..4 {
            wr_f32(&mut buf, 280 + (r * 4 + c) * 4, h.srow[r][c]);
        }
    }
    wr_cstr(&mut buf, 148, 80, &h.descrip);
    buf[143] = h.xyzt_units;
    buf[344..348].copy_from_slice(MAGIC_NIFTI_1);
    buf
}

/// A [`tpt_io_core::Writer`] for [`Nifti1Header`].
#[derive(Debug, Clone, Default)]
pub struct NiftiWriter;

impl NiftiWriter {
    /// Create a new writer.
    pub fn new() -> Self {
        Self
    }
}

impl tpt_io_core::Writer<Nifti1Header> for NiftiWriter {
    type Error = core::convert::Infallible;

    fn serialize(
        &self,
        value: &Nifti1Header,
    ) -> core::result::Result<Vec<u8>, core::convert::Infallible> {
        Ok(serialize_header(value))
    }
}

/// Convenience: round-trip a header through serialization for testing.
pub fn round_trip(h: &Nifti1Header) -> core::result::Result<Nifti1Header, NiftiError> {
    parse_header(&serialize_header(h))
}
