//! NetCDF file model and data-access helpers.

use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

use crate::error::NetCdfError;
use crate::header::{Attr, Dim, NcType, Var};
use tpt_io_core::ParseError;

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
    ///
    /// For record (unlimited-dimension) variables the on-disk layout
    /// interleaves records across all record variables, so a single contiguous
    /// slice is not meaningful — use [`NetCdf::read_hyperslab`] instead. This
    /// method therefore returns [`NetCdfError::Unsupported`] for record vars.
    pub fn var_data<'a>(
        &self,
        file: &'a [u8],
        v: &Var,
    ) -> core::result::Result<&'a [u8], NetCdfError> {
        if self.is_record_var(v) {
            return Err(NetCdfError::Unsupported(
                "record variables are interleaved on disk; use read_hyperslab",
            ));
        }
        let len = self.var_data_len(v);
        let start = v.begin as usize;
        let end = start + len;
        file.get(start..end).ok_or_else(|| NetCdfError::Core(ParseError::eof(end)))
    }

    /// The shape of a variable as its per-dimension lengths. The unlimited
    /// (record) dimension, if any, is resolved to its current record count.
    pub fn var_shape(&self, v: &Var) -> Vec<usize> {
        v.dim_ids.iter().map(|&d| self.dims[d as usize].len as usize).collect()
    }

    /// Whether `v` is a record variable (its first dimension is unlimited).
    pub fn is_record_var(&self, v: &Var) -> bool {
        v.dim_ids.first().is_some_and(|&d| self.dims[d as usize].unlimited)
    }

    /// Total bytes occupied by one record across *all* record variables. This is
    /// the stride between consecutive records of any record variable.
    fn record_block_size(&self) -> usize {
        self.vars
            .iter()
            .filter(|vv| self.is_record_var(vv))
            .map(|vv| {
                // `vlen` for a record variable spans *all* records; the per-record
                // slice is `vlen / record_count` (guarded against a zero count).
                let rcount = self.dims[vv.dim_ids[0] as usize].len as usize;
                (vv.vlen as usize).checked_div(rcount).unwrap_or(vv.vlen as usize)
            })
            .sum()
    }

    /// Read a single scalar element of `v` at absolute file `offset` as `f64`.
    fn read_scalar_at(&self, file: &[u8], v: &Var, offset: usize) -> Option<f64> {
        let esize = v.nc_type.size_of();
        let b = file.get(offset..offset + esize)?;
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

    /// Read a hyperslab (sub-array) of a variable without materializing the
    /// whole array — the core primitive for chunked reading of large
    /// `time × depth × lat × lon` grids.
    ///
    /// `start`/`count` are per-dimension index ranges (`start[i] + count[i] <=
    /// shape[i]`). Elements are returned in C/Fortran order with the last
    /// dimension varying fastest, matching NetCDF's on-disk layout, so they can
    /// be streamed straight into a lower-dimensional buffer.
    pub fn read_hyperslab(
        &self,
        file: &[u8],
        v: &Var,
        start: &[usize],
        count: &[usize],
    ) -> core::result::Result<Vec<f64>, NetCdfError> {
        let shape = self.var_shape(v);
        if start.len() != shape.len() || count.len() != shape.len() {
            return Err(NetCdfError::Core(ParseError::custom(
                0,
                "hyperslab rank does not match variable rank",
            )));
        }
        if start.iter().zip(count).zip(&shape).any(|((s, c), sh)| s.saturating_add(*c) > *sh) {
            return Err(NetCdfError::Core(ParseError::custom(
                0,
                "hyperslab extends past variable bounds",
            )));
        }
        let esize = v.nc_type.size_of();
        if esize == 0 {
            return Err(NetCdfError::Unsupported("cannot read variable of unknown type"));
        }

        let is_record = self.is_record_var(v);
        let rblock = if is_record { self.record_block_size() } else { 0 };
        let nd = shape.len();

        // Strides for the contiguous / within-record layout: last dim varies
        // fastest. `strides[k]` is the product of `shape[k+1..nd]`.
        let mut strides = vec![1usize; nd];
        let mut acc = 1usize;
        for (k, s) in strides.iter_mut().enumerate().rev() {
            *s = acc;
            acc = acc.saturating_mul(shape[k]);
        }

        let total: usize = count.iter().product();
        let mut out = Vec::with_capacity(total);
        if total == 0 {
            return Ok(out);
        }
        let mut pos = vec![0usize; nd];
        loop {
            let mut offset = v.begin as usize;
            if is_record {
                // Record variable: dimension 0 is the record axis; the remaining
                // axes are stored contiguously within each record.
                let r = start[0] + pos[0];
                offset += r * rblock;
                let within: usize = (1..nd).map(|i| (start[i] + pos[i]) * strides[i]).sum();
                offset += within * esize;
            } else {
                let lin: usize = (0..nd).map(|i| (start[i] + pos[i]) * strides[i]).sum();
                offset += lin * esize;
            }
            out.push(
                self.read_scalar_at(file, v, offset)
                    .ok_or_else(|| NetCdfError::Core(ParseError::eof(offset)))?,
            );

            // Advance the position vector, last dimension fastest.
            let mut i = nd;
            while i > 0 {
                i -= 1;
                pos[i] += 1;
                if pos[i] < count[i] {
                    break;
                }
                pos[i] = 0;
            }
            if i == 0 && pos[0] == 0 {
                break;
            }
        }
        Ok(out)
    }

    /// Variables whose name matches a dimension name are "coordinate
    /// variables" (they map an index axis to world coordinates). Return them.
    pub fn coordinate_variables(&self) -> Vec<&Var> {
        self.vars.iter().filter(|v| self.dims.iter().any(|d| d.name == v.name)).collect()
    }

    /// Read a single scalar from a variable at linear `index` (last dimension
    /// fastest) as `f64`. Record variables should use [`NetCdf::read_hyperslab`]
    /// instead, since their data is interleaved; this returns `None` for them.
    pub fn var_scalar(&self, file: &[u8], v: &Var, index: usize) -> Option<f64> {
        if self.is_record_var(v) {
            return None;
        }
        let esize = v.nc_type.size_of();
        let offset = v.begin as usize + index * esize;
        self.read_scalar_at(file, v, offset)
    }
}

/// Helper to build an attribute value from a slice of `f64` (for tests/writers).
pub fn attr_from_f64(_name: &str, _vals: &[f64]) -> Attr {
    Attr { name: String::new(), nc_type: NcType::Double, value: Vec::new() }
}
