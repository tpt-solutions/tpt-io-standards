//! NetCDF header serialization (round-trip).
//!
//! [`serialize_header`] rebuilds the NetCDF-3 header byte stream from a typed
//! [`NetCdf`]. The variable data section is not re-emitted (it is stored
//! separately), so a `parse → serialize → parse` cycle reproduces the header.

use alloc::vec;
use alloc::vec::Vec;

use crate::error::NetCdfError;
use crate::header::{parse, Attr, NetCdf, TAG_ATTRIBUTE, TAG_DIMENSION, TAG_VARIABLE};

fn wr_u32(buf: &mut Vec<u8>, v: u32) {
    buf.extend_from_slice(&v.to_be_bytes());
}
fn wr_i32(buf: &mut Vec<u8>, v: i32) {
    buf.extend_from_slice(&v.to_be_bytes());
}
fn wr_u64(buf: &mut Vec<u8>, v: u64) {
    buf.extend_from_slice(&v.to_be_bytes());
}
fn wr_str(buf: &mut Vec<u8>, s: &str) {
    let bytes = s.as_bytes();
    wr_u32(buf, bytes.len() as u32);
    buf.extend_from_slice(bytes);
    let padded = (bytes.len() + 3) & !3;
    buf.extend(vec![0u8; padded - bytes.len()]);
}
fn wr_attr(buf: &mut Vec<u8>, a: &Attr) {
    wr_str(buf, &a.name);
    wr_i32(buf, a.nc_type.code());
    wr_u32(buf, (a.value.len() / a.nc_type.size_of().max(1)) as u32);
    buf.extend_from_slice(&a.value);
    let padded = (a.value.len() + 3) & !3;
    buf.extend(vec![0u8; padded - a.value.len()]);
}

/// Serialize a typed NetCDF header back to bytes.
pub fn serialize_header(nc: &NetCdf) -> Vec<u8> {
    let mut buf = vec![];
    buf.extend_from_slice(b"CDF");
    buf.push(nc.version);
    let numrecs = nc.numrecs.unwrap_or(0xFFFF_FFFF);
    wr_u32(&mut buf, numrecs);

    wr_i32(&mut buf, TAG_DIMENSION);
    wr_u32(&mut buf, nc.dims.len() as u32);
    for d in &nc.dims {
        wr_str(&mut buf, &d.name);
        wr_u32(&mut buf, d.len);
    }

    wr_i32(&mut buf, TAG_ATTRIBUTE);
    wr_u32(&mut buf, nc.global_attrs.len() as u32);
    for a in &nc.global_attrs {
        wr_attr(&mut buf, a);
    }

    wr_i32(&mut buf, TAG_VARIABLE);
    wr_u32(&mut buf, nc.vars.len() as u32);
    for v in &nc.vars {
        wr_str(&mut buf, &v.name);
        wr_i32(&mut buf, v.nc_type.code());
        wr_u32(&mut buf, v.dim_ids.len() as u32);
        for &d in &v.dim_ids {
            wr_u32(&mut buf, d);
        }
        wr_u32(&mut buf, v.attrs.len() as u32);
        for a in &v.attrs {
            wr_attr(&mut buf, a);
        }
        if nc.version == 2 {
            wr_u64(&mut buf, v.vlen);
            wr_u64(&mut buf, v.begin);
        } else {
            wr_u32(&mut buf, v.vlen as u32);
            wr_u32(&mut buf, v.begin as u32);
        }
    }
    buf
}

/// Convenience: round-trip a header through serialization for testing.
pub fn round_trip(nc: &NetCdf) -> core::result::Result<NetCdf, NetCdfError> {
    parse(&serialize_header(nc))
}

/// A [`tpt_io_core::Writer`] for [`NetCdf`].
#[derive(Debug, Clone, Default)]
pub struct NetCdfWriter;

impl NetCdfWriter {
    /// Create a new writer.
    pub fn new() -> Self {
        Self
    }
}

impl tpt_io_core::Writer<NetCdf> for NetCdfWriter {
    type Error = core::convert::Infallible;

    fn serialize(
        &self,
        value: &NetCdf,
    ) -> core::result::Result<Vec<u8>, core::convert::Infallible> {
        Ok(serialize_header(value))
    }
}
