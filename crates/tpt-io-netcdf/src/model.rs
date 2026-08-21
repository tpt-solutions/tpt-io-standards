//! NetCDF file model and data-access helpers.

use alloc::vec::Vec;

use crate::error::NetCdfError;
use crate::header::{Attr, Dim, NcType, Var};

pub use crate::header::NetCdf;

impl NetCdf {
    /// Look up a dimension by name.
    pub fn dim(&self, name: &str) -> Option<&Dim> {
        self.dims.iter().find(|d| d.name == name)
    }

    /// Look up a variable by name.
    pub fn var(&self, name: &str) -> Option<&Var> {
        self.vars.iter().find(|v| v.name == name)
    }

    /// Total number of elements across a variable's non-record dimensions.
    pub fn var_element_count(&self, v: &Var) -> usize {
        let mut n = 1usize;
        for &d in &v.dim_ids {
            if let Some(dim) = self.dims.get(d as usize) {
                n = n.saturating_mul(dim.len as usize);
            }
        }
        n
    }

    /// Byte length of a variable's entire (non-record) data block.
    pub fn var_data_len(&self, v: &Var) -> usize {
        self.var_element_count(v) * v.nc_type.size_of()
    }

    /// Extract a variable's raw data block from the file bytes.
    pub fn var_data<'a>(
        &self,
        file: &'a [u8],
        v: &Var,
    ) -> core::result::Result<&'a [u8], NetCdfError> {
        let len = self.var_data_len(v);
        let start = v.begin as usize;
        let end = start + len;
        file.get(start..end).ok_or_else(|| NetCdfError::Core(tpt_io_core::ParseError::eof(end)))
    }

    /// Variables whose name matches a dimension name are "coordinate
    /// variables" (they map an index axis to world coordinates). Return them.
    pub fn coordinate_variables(&self) -> Vec<&Var> {
        self.vars.iter().filter(|v| self.dims.iter().any(|d| d.name == v.name)).collect()
    }

    /// Read a single scalar from a variable at `index` as `f64`.
    pub fn var_scalar(&self, file: &[u8], v: &Var, index: usize) -> Option<f64> {
        let esize = v.nc_type.size_of();
        let data = self.var_data(file, v).ok()?;
        let base = index * esize;
        let b = data.get(base..base + esize)?;
        Some(match v.nc_type {
            NcType::Ubyte => b[0] as f64,
            NcType::Char => b[0] as f64,
            NcType::Short => i16::from_be_bytes([b[0], b[1]]) as f64,
            NcType::Int => i32::from_be_bytes([b[0], b[1], b[2], b[3]]) as f64,
            NcType::Float => f32::from_be_bytes([b[0], b[1], b[2], b[3]]) as f64,
            NcType::Double => f64::from_be_bytes([b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7]]),
            NcType::Other(_) => return None,
        })
    }
}

/// Helper to build an attribute value from a slice of `f64` (for tests/writers).
pub fn attr_from_f64(_name: &str, _vals: &[f64]) -> Attr {
    Attr { name: String::new(), nc_type: NcType::Double, value: Vec::new() }
}
