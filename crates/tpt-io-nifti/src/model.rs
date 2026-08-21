//! NIfTI file model: a header plus the raw voxel-data bytes.

use alloc::vec::Vec;

use crate::coord::Point3;
use crate::error::NiftiError;
use crate::header::{DataType, Nifti1Header};

/// A parsed NIfTI-1 file: header and the raw voxel-data byte block.
#[derive(Debug, Clone, PartialEq)]
pub struct NiftiFile {
    /// Parsed header.
    pub header: Nifti1Header,
    /// Raw voxel data (exactly `header.data_bytes()` bytes).
    pub data: Vec<u8>,
}

impl NiftiFile {
    /// Number of voxels.
    pub fn voxel_count(&self) -> usize {
        self.header.voxel_count()
    }

    /// Read the scalar value of voxel `index` (0..voxel_count), applying
    /// `scl_slope`/`scl_inter`, returned as `f64`. Returns `None` on out-of-range
    /// or unknown datatype.
    pub fn voxel(&self, index: usize) -> Option<f64> {
        let esize = self.header.datatype.size_of();
        if esize == 0 || index >= self.voxel_count() {
            return None;
        }
        let base = index * esize;
        let bytes = self.data.get(base..base + esize)?;
        let raw = match self.header.datatype {
            DataType::Uint8 | DataType::Int8 => bytes[0] as f64,
            DataType::Int16 => i16::from_le_bytes([bytes[0], bytes[1]]) as f64,
            DataType::Uint16 => u16::from_le_bytes([bytes[0], bytes[1]]) as f64,
            DataType::Int32 => i32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as f64,
            DataType::Uint32 => u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as f64,
            DataType::Float32 => {
                f32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as f64
            }
            DataType::Float64 => f64::from_le_bytes([
                bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
            ]),
            DataType::Int64 => i64::from_le_bytes([
                bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
            ]) as f64,
            DataType::Uint64 => u64::from_le_bytes([
                bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
            ]) as f64,
            _ => return None,
        };
        Some(raw * self.header.scl_slope as f64 + self.header.scl_inter as f64)
    }

    /// World coordinate (RAS) of voxel center `(i, j, k)`.
    pub fn voxel_center_world(&self, i: f32, j: f32, k: f32) -> Point3 {
        crate::coord::voxel_to_world(&self.header.affine(), i, j, k)
    }
}

/// Build a [`NiftiFile`] from a header and a raw data block, validating lengths.
pub fn from_parts(
    header: Nifti1Header,
    data: Vec<u8>,
) -> core::result::Result<NiftiFile, NiftiError> {
    let expected = header.data_bytes();
    if data.len() != expected {
        return Err(NiftiError::Core(tpt_io_core::ParseError::custom(
            0,
            alloc::format!("voxel data length {} does not match expected {}", data.len(), expected),
        )));
    }
    Ok(NiftiFile { header, data })
}
