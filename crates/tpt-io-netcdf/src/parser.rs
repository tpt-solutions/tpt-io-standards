//! NetCDF reader, validator, and high-level parse entry points.

use tpt_io_core::{ParseError, Reader, Validator};

use crate::error::NetCdfError;
use crate::header::parse;
use crate::model::NetCdf;

/// A NetCDF reader implementing [`tpt_io_core::Reader`].
#[derive(Debug, Clone, Default)]
pub struct NetCdfReader;

impl NetCdfReader {
    /// Create a new reader.
    pub fn new() -> Self {
        Self
    }
}

impl Reader<NetCdf> for NetCdfReader {
    type Error = NetCdfError;

    fn parse(&self, input: &[u8]) -> core::result::Result<NetCdf, NetCdfError> {
        parse_netcdf(input)
    }
}

/// Parse a NetCDF-3 (classic or 64-bit) byte stream into a [`NetCdf`].
pub fn parse_netcdf(data: &[u8]) -> core::result::Result<NetCdf, NetCdfError> {
    parse(data)
}

/// NetCDF syntactic/semantic validator.
#[derive(Debug, Clone, Default)]
pub struct NetCdfValidator;

impl NetCdfValidator {
    /// Create a new validator.
    pub fn new() -> Self {
        Self
    }
}

impl Validator<NetCdf> for NetCdfValidator {
    type Error = NetCdfError;

    fn validate_syntactic(&self, value: &NetCdf) -> core::result::Result<(), NetCdfError> {
        for v in &value.vars {
            for &d in &v.dim_ids {
                if (d as usize) >= value.dims.len() {
                    return Err(NetCdfError::Core(ParseError::custom(
                        0,
                        "variable references undefined dimension",
                    )));
                }
            }
        }
        Ok(())
    }

    fn validate_semantic(&self, value: &NetCdf) -> core::result::Result<(), NetCdfError> {
        for v in &value.vars {
            if v.nc_type.size_of() == 0 {
                return Err(NetCdfError::Unsupported("variable has unknown type"));
            }
        }
        Ok(())
    }
}
