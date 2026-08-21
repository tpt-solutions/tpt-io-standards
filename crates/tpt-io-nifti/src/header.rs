//! NIfTI-1 header parsing.
//!
//! NIfTI-1 stores a fixed 348-byte header (little-endian) followed by optional
//! extensions and then the voxel data at byte offset `vox_offset`. This module
//! reads the header fields into a typed [`Nifti1Header`] and exposes the
//! metadata needed by the rest of the crate (dimensions, voxel sizes, datatype,
//! orientation via `sform`/`qform`, and the affine transform).
//!
//! Reading is `no_std`-safe and uses no floating-point bit-twiddling beyond
//! [`f32::from_le_bytes`].

use alloc::string::String;
use alloc::vec::Vec;

use crate::error::NiftiError;
use tpt_io_core::ParseError;

/// Magic string identifying a NIfTI-1 file (`"n+1\0"`).
pub const MAGIC_NIFTI_1: &[u8; 4] = b"n+1\0";
/// Magic string identifying a NIfTI-2 file (`"ni2\0"`).
pub const MAGIC_NIFTI_2: &[u8; 4] = b"ni2\0";

/// NIfTI datatype codes (subset of the common ones).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataType {
    /// Unknown / unspecified.
    Unknown,
    /// Unsigned 8-bit integer.
    Uint8,
    /// Signed 8-bit integer.
    Int8,
    /// Signed 16-bit integer.
    Int16,
    /// Unsigned 16-bit integer.
    Uint16,
    /// Signed 32-bit integer.
    Int32,
    /// Unsigned 32-bit integer.
    Uint32,
    /// 32-bit float.
    Float32,
    /// 64-bit float.
    Float64,
    /// Signed 64-bit integer.
    Int64,
    /// Unsigned 64-bit integer.
    Uint64,
    /// Complex 64-bit float.
    Complex64,
    /// 128-bit float.
    Float128,
    /// A datatype code not enumerated here.
    Other(i16),
}

impl DataType {
    /// Map a NIfTI datatype code to [`DataType`].
    pub fn from_code(code: i16) -> DataType {
        match code {
            0 => DataType::Unknown,
            2 => DataType::Uint8,
            4 => DataType::Int16,
            8 => DataType::Int32,
            16 => DataType::Float32,
            32 => DataType::Complex64,
            64 => DataType::Float64,
            128 => DataType::Uint8,
            256 => DataType::Int8,
            512 => DataType::Uint16,
            768 => DataType::Uint32,
            1024 => DataType::Int64,
            1280 => DataType::Uint64,
            1536 => DataType::Float128,
            other => DataType::Other(other),
        }
    }

    /// The NIfTI datatype code for this variant.
    pub fn code(&self) -> i16 {
        match self {
            DataType::Unknown => 0,
            DataType::Uint8 => 2,
            DataType::Int16 => 4,
            DataType::Int32 => 8,
            DataType::Float32 => 16,
            DataType::Complex64 => 32,
            DataType::Float64 => 64,
            DataType::Int8 => 256,
            DataType::Uint16 => 512,
            DataType::Uint32 => 768,
            DataType::Int64 => 1024,
            DataType::Uint64 => 1280,
            DataType::Float128 => 1536,
            DataType::Other(c) => *c,
        }
    }

    /// Number of bits per voxel element.
    pub fn bitpix(&self) -> u16 {
        match self {
            DataType::Uint8 | DataType::Int8 => 8,
            DataType::Int16 | DataType::Uint16 => 16,
            DataType::Int32 | DataType::Uint32 | DataType::Float32 | DataType::Complex64 => 32,
            DataType::Int64 | DataType::Uint64 | DataType::Float64 => 64,
            DataType::Float128 => 128,
            DataType::Unknown | DataType::Other(_) => 0,
        }
    }

    /// Number of bytes per voxel element (0 if unknown).
    pub fn size_of(&self) -> usize {
        (self.bitpix() as usize) / 8
    }
}

/// A parsed NIfTI-1 header.
#[derive(Debug, Clone, PartialEq)]
pub struct Nifti1Header {
    /// Raw 348-byte header (owned for round-trip and `no_std` friendliness).
    pub raw: Vec<u8>,
    /// NIfTI version detected (1).
    pub version: u8,
    /// Number of dimensions (1..7).
    pub ndim: u8,
    /// Size of each dimension (indices 0..ndim-1 meaningful).
    pub dim: [u16; 7],
    /// Voxel sizes for each dimension (pixdim[1..]).
    pub pixdim: [f32; 7],
    /// Datatype code.
    pub datatype: DataType,
    /// Bits per element.
    pub bitpix: u16,
    /// Byte offset of the voxel data from the start of the file.
    pub vox_offset: u32,
    /// Scale slope applied to stored values (`stored * slope + inter`).
    pub scl_slope: f32,
    /// Scale intercept applied to stored values.
    pub scl_inter: f32,
    /// `sform` code (method used to compute the affine).
    pub sform_code: i16,
    /// `qform` code.
    pub qform_code: i16,
    /// Quaternion parameters (b, c, d) for the `qform` (if used).
    pub quatern: [f32; 3],
    /// Quaternion offset (x, y, z) for the `qform`.
    pub qoffset: [f32; 3],
    /// The `sform` 3x4 affine rows (x, y, z).
    pub srow: [[f32; 4]; 3],
    /// Free-text description.
    pub descrip: String,
    /// Measurement units: spatial (low 3 bits) and time (next 3 bits).
    pub xyzt_units: u8,
}

impl Nifti1Header {
    /// Total number of voxels (`dim[0]*...*dim[ndim-1]`).
    pub fn voxel_count(&self) -> usize {
        let mut n = 1usize;
        for i in 0..self.ndim as usize {
            n = n.saturating_mul(self.dim[i] as usize);
        }
        n
    }

    /// Total number of bytes occupied by the voxel data.
    pub fn data_bytes(&self) -> usize {
        self.voxel_count().saturating_mul(self.datatype.size_of())
    }

    /// Extract the 4x4 affine matrix (row-major). Falls back to `qform` when
    /// `sform` is not set, and to a diag(pixdim) identity-ish matrix otherwise.
    pub fn affine(&self) -> [[f32; 4]; 4] {
        if self.sform_code > 0 {
            let mut m = [[0f32; 4]; 4];
            for r in 0..3 {
                for c in 0..4 {
                    m[r][c] = self.srow[r][c];
                }
            }
            m[3] = [0.0, 0.0, 0.0, 1.0];
            m
        } else if self.qform_code > 0 {
            qform_affine(self)
        } else {
            let mut m = [[0f32; 4]; 4];
            for i in 0..3 {
                m[i][i] = self.pixdim[i];
            }
            m[3] = [0.0, 0.0, 0.0, 1.0];
            m
        }
    }
}

fn qform_affine(h: &Nifti1Header) -> [[f32; 4]; 4] {
    let [b, c, d] = h.quatern;
    let a = (1.0 - b * b - c * c - d * d).sqrt();
    let [qx, qy, qz] = h.qoffset;
    // Rotation matrix from quaternion (a,b,c,d). NIfTI convention.
    let r00 = a * a + b * b - c * c - d * d;
    let r01 = 2.0 * (b * c - a * d);
    let r02 = 2.0 * (b * d + a * c);
    let r10 = 2.0 * (b * c + a * d);
    let r11 = a * a + c * c - b * b - d * d;
    let r12 = 2.0 * (c * d - a * b);
    let r20 = 2.0 * (b * d - a * c);
    let r21 = 2.0 * (c * d + a * b);
    let r22 = a * a + d * d - b * b - c * c;
    let qfac = if h.pixdim[0].abs() < 1e-9 { 1.0 } else { h.pixdim[0] };
    let mut m = [[0f32; 4]; 4];
    m[0][0] = r00 * h.pixdim[1];
    m[0][1] = r01 * h.pixdim[2];
    m[0][2] = r02 * h.pixdim[3] * qfac;
    m[0][3] = qx;
    m[1][0] = r10 * h.pixdim[1];
    m[1][1] = r11 * h.pixdim[2];
    m[1][2] = r12 * h.pixdim[3] * qfac;
    m[1][3] = qy;
    m[2][0] = r20 * h.pixdim[1];
    m[2][1] = r21 * h.pixdim[2];
    m[2][2] = r22 * h.pixdim[3] * qfac;
    m[2][3] = qz;
    m[3] = [0.0, 0.0, 0.0, 1.0];
    m
}

/// Read an `i16` at `offset` (little-endian).
pub fn rd_i16(b: &[u8], offset: usize) -> Result<i16, ParseError> {
    let s = b.get(offset..offset + 2).ok_or_else(|| ParseError::eof(offset))?;
    Ok(i16::from_le_bytes([s[0], s[1]]))
}

/// Read a `u16` at `offset` (little-endian).
pub fn rd_u16(b: &[u8], offset: usize) -> Result<u16, ParseError> {
    let s = b.get(offset..offset + 2).ok_or_else(|| ParseError::eof(offset))?;
    Ok(u16::from_le_bytes([s[0], s[1]]))
}

/// Read an `i32` at `offset` (little-endian).
pub fn rd_i32(b: &[u8], offset: usize) -> Result<i32, ParseError> {
    let s = b.get(offset..offset + 4).ok_or_else(|| ParseError::eof(offset))?;
    Ok(i32::from_le_bytes([s[0], s[1], s[2], s[3]]))
}

/// Read an `f32` at `offset` (little-endian).
pub fn rd_f32(b: &[u8], offset: usize) -> Result<f32, ParseError> {
    let s = b.get(offset..offset + 4).ok_or_else(|| ParseError::eof(offset))?;
    Ok(f32::from_le_bytes([s[0], s[1], s[2], s[3]]))
}

/// Read a fixed-length, NUL-terminated ASCII string field.
pub fn rd_cstr(b: &[u8], offset: usize, len: usize) -> Result<String, ParseError> {
    let s = b.get(offset..offset + len).ok_or_else(|| ParseError::eof(offset))?;
    let end = s.iter().position(|&c| c == 0).unwrap_or(len);
    Ok(String::from_utf8_lossy(&s[..end]).into_owned())
}

/// Parse a NIfTI-1 header from the first 348 bytes of `data`.
pub fn parse_header(data: &[u8]) -> core::result::Result<Nifti1Header, NiftiError> {
    if data.len() < 348 {
        return Err(NiftiError::Core(ParseError::eof(data.len())));
    }
    let sizeof_hdr = rd_i32(data, 0).map_err(NiftiError::Core)?;
    if sizeof_hdr != 348 {
        return Err(NiftiError::Unsupported("not a NIfTI-1 header (sizeof_hdr != 348)"));
    }
    let magic = &data[344..348];
    let (version, _) = if magic == MAGIC_NIFTI_1 {
        (1u8, true)
    } else if magic == MAGIC_NIFTI_2 {
        return Err(NiftiError::Unsupported("NIfTI-2 not yet supported"));
    } else {
        return Err(NiftiError::Unsupported("missing n+1 magic"));
    };

    let ndim_i = rd_i16(data, 40).map_err(NiftiError::Core)?;
    let mut dim = [0u16; 7];
    let mut pixdim = [0f32; 7];
    for i in 0..7 {
        dim[i] = rd_i16(data, 42 + i * 2).map_err(NiftiError::Core)? as u16;
        pixdim[i] = rd_f32(data, 76 + i * 4).map_err(NiftiError::Core)?;
    }
    let datatype_code = rd_i16(data, 70).map_err(NiftiError::Core)?;
    let bitpix = rd_i16(data, 72).map_err(NiftiError::Core)? as u16;
    let vox_offset = rd_f32(data, 108).map_err(NiftiError::Core)? as u32;
    let scl_slope = rd_f32(data, 112).map_err(NiftiError::Core)?;
    let scl_slope = if scl_slope == 0.0 { 1.0 } else { scl_slope };
    let scl_inter = rd_f32(data, 116).map_err(NiftiError::Core)?;
    let sform_code = rd_i16(data, 252).map_err(NiftiError::Core)?;
    let qform_code = rd_i16(data, 252 - 2).map_err(NiftiError::Core)?;
    let quatern = [
        rd_f32(data, 256).map_err(NiftiError::Core)?,
        rd_f32(data, 260).map_err(NiftiError::Core)?,
        rd_f32(data, 264).map_err(NiftiError::Core)?,
    ];
    let qoffset = [
        rd_f32(data, 268).map_err(NiftiError::Core)?,
        rd_f32(data, 272).map_err(NiftiError::Core)?,
        rd_f32(data, 276).map_err(NiftiError::Core)?,
    ];
    let mut srow = [[0f32; 4]; 3];
    for r in 0..3 {
        for c in 0..4 {
            srow[r][c] = rd_f32(data, 280 + (r * 4 + c) * 4).map_err(NiftiError::Core)?;
        }
    }
    let descrip = rd_cstr(data, 148, 80).map_err(NiftiError::Core)?;
    let xyzt_units = data[143];

    Ok(Nifti1Header {
        raw: data[..348].to_vec(),
        version,
        ndim: (ndim_i as u8).clamp(1, 7),
        dim,
        pixdim,
        datatype: DataType::from_code(datatype_code),
        bitpix,
        vox_offset,
        scl_slope,
        scl_inter,
        sform_code,
        qform_code,
        quatern,
        qoffset,
        srow,
        descrip,
        xyzt_units,
    })
}
