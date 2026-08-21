//! NetCDF-3 classic header parser.
//!
//! A NetCDF-3 file starts with the magic `CDF` followed by a version byte
//! (`1` = classic/32-bit offsets, `2` = 64-bit offsets). The rest of the header
//! is a sequence of tagged sections: dimensions, global attributes, and
//! variables (each variable carrying its own attributes). This module decodes
//! that into a typed [`NetCdf`] alongside the data section offsets.

use alloc::string::String;
use alloc::vec::Vec;

use crate::error::NetCdfError;
use tpt_io_core::ParseError;

/// NetCDF `nc_type` codes (classic subset).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NcType {
    /// 8-bit unsigned integer.
    Ubyte,
    /// 8-bit character.
    Char,
    /// 16-bit signed integer.
    Short,
    /// 32-bit signed integer.
    Int,
    /// 32-bit float.
    Float,
    /// 64-bit float.
    Double,
    /// A type code not enumerated here.
    Other(i32),
}

impl NcType {
    /// Map a NetCDF type code to [`NcType`].
    pub fn from_code(c: i32) -> NcType {
        match c {
            1 => NcType::Ubyte,
            2 => NcType::Char,
            3 => NcType::Short,
            4 => NcType::Int,
            5 => NcType::Float,
            6 => NcType::Double,
            other => NcType::Other(other),
        }
    }

    /// The NetCDF type code for this variant.
    pub fn code(&self) -> i32 {
        match self {
            NcType::Ubyte => 1,
            NcType::Char => 2,
            NcType::Short => 3,
            NcType::Int => 4,
            NcType::Float => 5,
            NcType::Double => 6,
            NcType::Other(c) => *c,
        }
    }

    /// Size in bytes of one element.
    pub fn size_of(&self) -> usize {
        match self {
            NcType::Ubyte | NcType::Char => 1,
            NcType::Short => 2,
            NcType::Int | NcType::Float => 4,
            NcType::Double => 8,
            NcType::Other(_) => 0,
        }
    }
}

/// A dimension definition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dim {
    /// Dimension name.
    pub name: String,
    /// Length (number of elements).
    pub len: u32,
}

/// An attribute (name + typed value bytes).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Attr {
    /// Attribute name.
    pub name: String,
    /// Attribute type.
    pub nc_type: NcType,
    /// Raw value bytes (length is `nc_type.size_of() * nelems`, unpadded).
    pub value: Vec<u8>,
}

/// A variable definition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Var {
    /// Variable name.
    pub name: String,
    /// Variable type.
    pub nc_type: NcType,
    /// Dimension ids this variable is defined over (in order).
    pub dim_ids: Vec<u32>,
    /// Per-variable attributes.
    pub attrs: Vec<Attr>,
    /// Record size in bytes (product of non-record dim sizes * elsize); 0 if
    /// the variable is not a record variable.
    pub vlen: u64,
    /// Byte offset of the variable's data in the file.
    pub begin: u64,
}

/// A fully parsed NetCDF-3 file (header + offsets; data lives in the raw bytes).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NetCdf {
    /// Version byte (`1` classic, `2` 64-bit offsets).
    pub version: u8,
    /// Number of records (or `None` if the file is a stream/unlimited-unknown).
    pub numrecs: Option<u32>,
    /// Dimension definitions.
    pub dims: Vec<Dim>,
    /// Global (file-level) attributes.
    pub global_attrs: Vec<Attr>,
    /// Variable definitions.
    pub vars: Vec<Var>,
    /// Length of the parsed header in bytes (start of the data section).
    pub header_len: usize,
}

pub const TAG_DIMENSION: i32 = 10;
pub const TAG_VARIABLE: i32 = 11;
pub const TAG_ATTRIBUTE: i32 = 12;

/// A cursor advancing through a NetCDF byte stream.
struct Cur<'a> {
    data: &'a [u8],
    pos: usize,
    wide: bool, // 8-byte offsets (version 2)
}

impl<'a> Cur<'a> {
    fn u8(&mut self) -> Result<u8, NetCdfError> {
        let b = *self.data.get(self.pos).ok_or_else(|| NetCdfError::Core(ParseError::eof(self.pos)))?;
        self.pos += 1;
        Ok(b)
    }
    fn u32(&mut self) -> Result<u32, NetCdfError> {
        let s = self.data.get(self.pos..self.pos + 4).ok_or_else(|| NetCdfError::Core(ParseError::eof(self.pos)))?;
        self.pos += 4;
        Ok(u32::from_be_bytes([s[0], s[1], s[2], s[3]]))
    }
    fn u64(&mut self) -> Result<u64, NetCdfError> {
        let s = self.data.get(self.pos..self.pos + 8).ok_or_else(|| NetCdfError::Core(ParseError::eof(self.pos)))?;
        self.pos += 8;
        Ok(u64::from_be_bytes([s[0], s[1], s[2], s[3], s[4], s[5], s[6], s[7]]))
    }
    fn i32(&mut self) -> Result<i32, NetCdfError> {
        Ok(self.u32()? as i32)
    }
    /// Read a length-prefixed, 4-byte-padded string.
    fn string(&mut self) -> Result<String, NetCdfError> {
        let n = self.u32()? as usize;
        let s = self.data.get(self.pos..self.pos + n).ok_or_else(|| NetCdfError::Core(ParseError::eof(self.pos)))?;
        self.pos += n;
        // Pad to a 4-byte boundary.
        let padded = (n + 3) & !3;
        self.pos += padded - n;
        Ok(String::from_utf8_lossy(s).into_owned())
    }
    /// Read `len` bytes (padded to 4) as an attribute value.
    fn value_bytes(&mut self, len: usize) -> Result<Vec<u8>, NetCdfError> {
        let s = self.data.get(self.pos..self.pos + len).ok_or_else(|| NetCdfError::Core(ParseError::eof(self.pos)))?;
        self.pos += len;
        let padded = (len + 3) & !3;
        self.pos += padded - len;
        Ok(s.to_vec())
    }
    fn offset(&mut self) -> Result<u64, NetCdfError> {
        if self.wide {
            self.u64()
        } else {
            Ok(self.u32()? as u64)
        }
    }
}

fn parse_attr(c: &mut Cur) -> Result<Attr, NetCdfError> {
    let name = c.string()?;
    let nc_type = NcType::from_code(c.i32()?);
    let nelems = c.u32()? as usize;
    let vlen = nelems * nc_type.size_of();
    let value = c.value_bytes(vlen)?;
    Ok(Attr { name, nc_type, value })
}

fn parse_var(c: &mut Cur) -> Result<Var, NetCdfError> {
    let name = c.string()?;
    let nc_type = NcType::from_code(c.i32()?);
    let nvdims = c.u32()? as usize;
    let mut dim_ids = Vec::with_capacity(nvdims);
    for _ in 0..nvdims {
        dim_ids.push(c.u32()?);
    }
    let nattrs = c.u32()? as usize;
    let mut attrs = Vec::with_capacity(nattrs);
    for _ in 0..nattrs {
        attrs.push(parse_attr(c)?);
    }
    let vlen = c.offset()?;
    let begin = c.offset()?;
    Ok(Var { name, nc_type, dim_ids, attrs, vlen, begin })
}

/// Parse a NetCDF-3 header (classic or 64-bit) from `data`.
pub fn parse(data: &[u8]) -> core::result::Result<NetCdf, NetCdfError> {
    if data.len() < 4 || &data[0..3] != b"CDF" {
        return Err(NetCdfError::Unsupported("not a NetCDF file (missing CDF magic)"));
    }
    let version = data[3];
    let wide = version == 2;
    if version != 1 && version != 2 {
        return Err(NetCdfError::Unsupported("only NetCDF-3 classic (version 1/2) supported"));
    }
    let mut c = Cur { data, pos: 4, wide };

    let numrecs_raw = c.u32()?;
    let numrecs = if numrecs_raw == 0xFFFF_FFFF {
        None
    } else {
        Some(numrecs_raw)
    };

    let mut dims = Vec::new();
    let mut global_attrs = Vec::new();
    let mut vars = Vec::new();

    // The header is a flat sequence of tags until the data section begins.
    loop {
        // Peek the next tag (i32).
        if c.pos + 4 > data.len() {
            break;
        }
        let tag = c.i32()?;
        match tag {
            TAG_DIMENSION => {
                let nelems = c.u32()? as usize;
                for _ in 0..nelems {
                    let name = c.string()?;
                    let len = c.u32()?;
                    dims.push(Dim { name, len });
                }
            }
            TAG_ATTRIBUTE => {
                let nelems = c.u32()? as usize;
                for _ in 0..nelems {
                    global_attrs.push(parse_attr(&mut c)?);
                }
            }
            TAG_VARIABLE => {
                let nelems = c.u32()? as usize;
                for _ in 0..nelems {
                    vars.push(parse_var(&mut c)?);
                }
            }
            other => {
                return Err(NetCdfError::Unsupported("unexpected header tag"));
            }
        }
        // Heuristic stop: once we have parsed dims/attrs/vars and the cursor has
        // clearly moved into the data region, break. We rely on well-formed
        // ordering (dims, then attrs, then vars) and stop after variables.
        if !vars.is_empty() {
            break;
        }
    }

    Ok(NetCdf { version, numrecs, dims, global_attrs, vars, header_len: c.pos })
}
