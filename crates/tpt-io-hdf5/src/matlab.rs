//! MATLAB v7.3 `.mat` files: HDF5 containers with MATLAB class attributes.
//!
//! A v7.3 file is an HDF5 file behind a 512-byte user block that starts with the text `MATLAB 7.3 MAT-file`.
//! Each variable is a dataset with a `MATLAB_class` attribute (`double`, `char`, `int32`, and so on).
//! [`parse_mat73`] finds the HDF5 superblock behind the user block and returns the variables. MATLAB stores arrays
//! column-major, so each variable's dimensions are the reverse of its HDF5 shape. Struct and cell variables are HDF5
//! groups, which are not returned.
//!
//! Spec access: the MAT-file format is documented by MathWorks, and the v7.3 layer is plain HDF5. No specification
//! text is copied into this crate.

use alloc::string::{String, ToString};
use alloc::vec::Vec;

use crate::{is_hdf5, parse, DecodedData, H5Attr, Hdf5Error};

/// The text at the start of a MATLAB v7.3 file.
pub const MARKER: &[u8] = b"MATLAB 7.3 MAT-file";

/// A MATLAB v7.3 file: its header text and its array variables.
#[derive(Debug, Clone, PartialEq)]
pub struct MatFile {
    /// The header text, such as `MATLAB 7.3 MAT-file, Platform: PCWIN64, ...`.
    pub description: String,
    /// The array variables, in file order.
    pub variables: Vec<MatVariable>,
}

/// One array variable.
#[derive(Debug, Clone, PartialEq)]
pub struct MatVariable {
    /// The variable name.
    pub name: String,
    /// The `MATLAB_class` attribute, such as `double`, `char` or `int32`.
    pub class: String,
    /// The MATLAB dimensions, column-major order (the reverse of the HDF5 shape).
    pub dims: Vec<u64>,
    /// The elements: `F64` for floating-point data, or the raw little-endian bytes for other types.
    pub data: DecodedData,
}

impl MatVariable {
    /// The elements of a `double` or `single` variable, widened to `f64`. `None` for other classes.
    pub fn as_f64(&self) -> Option<Vec<f64>> {
        let DecodedData::Raw(bytes) = &self.data else { return None };
        match self.class.as_str() {
            "double" => Some(
                bytes
                    .chunks_exact(8)
                    .map(|c| {
                        let mut b = [0u8; 8];
                        b.copy_from_slice(c);
                        f64::from_le_bytes(b)
                    })
                    .collect(),
            ),
            "single" => Some(
                bytes
                    .chunks_exact(4)
                    .map(|c| {
                        let mut b = [0u8; 4];
                        b.copy_from_slice(c);
                        f32::from_le_bytes(b) as f64
                    })
                    .collect(),
            ),
            _ => None,
        }
    }
}

/// Whether `data` starts with the MATLAB v7.3 header text.
pub fn is_mat73(data: &[u8]) -> bool {
    data.starts_with(MARKER)
}

/// Read a MATLAB v7.3 file and return its header text and array variables.
pub fn parse_mat73(data: &[u8]) -> Result<MatFile, Hdf5Error> {
    if !is_mat73(data) {
        return Err(Hdf5Error::Unsupported("not a MATLAB 7.3 file"));
    }
    let description =
        String::from_utf8_lossy(data.get(..116).unwrap_or(data)).trim_end().to_string();
    let start = superblock_offset(data)
        .ok_or(Hdf5Error::Unsupported("no HDF5 superblock after the MATLAB user block"))?;
    let body = &data[start..];
    let file = parse(body)?;
    let mut variables = Vec::new();
    for ds in &file.datasets {
        let Some(class) = class_of(&ds.attrs) else { continue };
        variables.push(MatVariable {
            name: ds.name.clone(),
            class,
            dims: ds.dims.iter().rev().copied().collect(),
            data: ds.read_data(body)?,
        });
    }
    Ok(MatFile { description, variables })
}

/// The offset of the HDF5 superblock. It sits after the 512-byte user block, at 512 or at a power of two beyond it.
fn superblock_offset(data: &[u8]) -> Option<usize> {
    let mut off = 512usize;
    while off < data.len() {
        if is_hdf5(&data[off..]) {
            return Some(off);
        }
        off *= 2;
    }
    None
}

fn class_of(attrs: &[H5Attr]) -> Option<String> {
    let a = attrs.iter().find(|a| a.name == "MATLAB_class")?;
    Some(String::from_utf8_lossy(&a.value).trim_end_matches('\0').to_string())
}
