//! ASAM MDF 4 measurement files: channel groups, channels, and their sample values.
//!
//! An MDF 4 file is a tree of blocks. Each block has a 24-byte header, followed by links (file offsets of other
//! blocks) and data. [`parse`] follows the links from the header to the data groups, the channel groups in each
//! group, and the channels in each channel group. It then decodes the records in the data blocks, which may be
//! stored as plain (`DT`), linked (`DL`), or deflate-compressed (`DZ`) blocks.
//!
//! Channel values come back as `f64`. Integer and floating-point channels are read; string and byte-array
//! channels are skipped. Conversions (the `CC` blocks) are not applied, so the values are the raw ones.
//!
//! ```
//! // A file with no channels is still a valid MDF 4 file; `parse` returns no groups for it.
//! let not_mdf = tpt_io_mdf::parse(b"plain text, not MDF");
//! assert!(not_mdf.is_err());
//! ```
//!
//! Spec access: the MDF 4 format is specified by ASAM (MDF 4.1) and ASAM publishes it openly. No specification text
//! is copied into this crate.

#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]

#[cfg(not(any(feature = "alloc", feature = "std")))]
compile_error!(
    "tpt-io-mdf requires the `alloc` or `std` feature: its channels own names and values."
);

#[cfg(any(feature = "alloc", feature = "std"))]
#[allow(unused_imports)]
#[macro_use]
extern crate alloc;

use alloc::string::{String, ToString};
use alloc::vec::Vec;

use tpt_io_core::inflate::inflate;
use tpt_io_core::ParseError;

/// Errors produced while reading an MDF 4 file.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum MdfError {
    /// A truncated or malformed block, with its byte offset.
    Core(ParseError),
    /// The file breaks a rule (not an MDF file, a block of the wrong type, a bad link).
    Invalid(String),
    /// A feature this reader does not implement, such as big-endian files or transposed compression.
    Unsupported(&'static str),
}

impl From<ParseError> for MdfError {
    fn from(e: ParseError) -> Self {
        MdfError::Core(e)
    }
}

impl core::fmt::Display for MdfError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            MdfError::Core(e) => write!(f, "{e}"),
            MdfError::Invalid(what) => write!(f, "invalid MDF file: {what}"),
            MdfError::Unsupported(what) => write!(f, "MDF unsupported: {what}"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for MdfError {}

/// A parsed MDF 4 file: its version and its channel groups, in file order.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Mdf {
    /// The format version from the identification block, such as `4.10`.
    pub version: String,
    /// The channel groups, across every data group.
    pub groups: Vec<ChannelGroup>,
}

/// A channel group: the channels that share one record layout.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ChannelGroup {
    /// The acquisition name, when the group has one.
    pub name: Option<String>,
    /// The number of records (cycles) the group holds.
    pub records: u64,
    /// The channels, in file order.
    pub channels: Vec<Channel>,
}

/// One channel: its name and unit, and its values, one per record.
#[derive(Debug, Clone, PartialEq)]
pub struct Channel {
    /// The channel name.
    pub name: String,
    /// The unit, when the channel has one.
    pub unit: Option<String>,
    /// The raw values, one per record.
    pub values: Vec<f64>,
}

/// A block read from the file: its type, its links, and its data bytes.
struct Block<'a> {
    kind: [u8; 4],
    links: Vec<u64>,
    data: &'a [u8],
}

fn le_u64(input: &[u8], at: usize) -> Result<u64, MdfError> {
    let s = input.get(at..at + 8).ok_or_else(|| ParseError::eof(at))?;
    let mut b = [0u8; 8];
    b.copy_from_slice(s);
    Ok(u64::from_le_bytes(b))
}

fn le_u32(input: &[u8], at: usize) -> Result<u32, MdfError> {
    let s = input.get(at..at + 4).ok_or_else(|| ParseError::eof(at))?;
    Ok(u32::from_le_bytes([s[0], s[1], s[2], s[3]]))
}

fn block(input: &[u8], at: u64) -> Result<Block<'_>, MdfError> {
    let at = at as usize;
    let header = input.get(at..at + 24).ok_or_else(|| ParseError::eof(at))?;
    let kind = [header[0], header[1], header[2], header[3]];
    let length = le_u64(input, at + 8)? as usize;
    let nlinks = le_u64(input, at + 16)? as usize;
    if length < 24 + 8 * nlinks {
        return Err(MdfError::Invalid("block is shorter than its header and links".to_string()));
    }
    let end =
        at.checked_add(length).filter(|&e| e <= input.len()).ok_or_else(|| ParseError::eof(at))?;
    let mut links = Vec::with_capacity(nlinks);
    for i in 0..nlinks {
        links.push(le_u64(input, at + 24 + 8 * i)?);
    }
    Ok(Block { kind, links, data: &input[at + 24 + 8 * nlinks..end] })
}

fn expect(block: &Block<'_>, kind: &[u8; 4], what: &'static str) -> Result<(), MdfError> {
    if &block.kind != kind {
        return Err(MdfError::Invalid(alloc::format!("expected a {what} block")));
    }
    Ok(())
}

/// The text of a `TX` or `MD` block: NUL-terminated, UTF-8.
fn text(input: &[u8], addr: u64) -> Result<String, MdfError> {
    if addr == 0 {
        return Ok(String::new());
    }
    let b = block(input, addr)?;
    let end = b.data.iter().position(|&c| c == 0).unwrap_or(b.data.len());
    Ok(String::from_utf8_lossy(&b.data[..end]).into_owned())
}

/// Read a file's data blocks as one byte stream: a `DT` directly, a `DZ` inflated, or a `DL` list of them.
fn data_bytes(input: &[u8], addr: u64, out: &mut Vec<u8>, depth: usize) -> Result<(), MdfError> {
    if depth > 8 {
        return Err(MdfError::Invalid("data list nests too deeply".to_string()));
    }
    let b = block(input, addr)?;
    match &b.kind {
        b"##DT" => out.extend_from_slice(b.data),
        b"##DZ" => {
            if b.data.len() < 24 {
                return Err(MdfError::Invalid("compressed data block is too short".to_string()));
            }
            // Zip type 0 is plain deflate; zip type 1 deflates a transposed block, whose record width is the
            // zip parameter.
            let transposed = match b.data[2] {
                0 => false,
                1 => true,
                _ => return Err(MdfError::Unsupported("unknown compressed data block type")),
            };
            let width = le_u32(b.data, 4)? as usize;
            let zip_len = le_u64(b.data, 16)? as usize;
            let packed =
                b.data.get(24..24 + zip_len).ok_or_else(|| ParseError::eof(addr as usize))?;
            // DZ blocks hold zlib streams. Strip the two-byte zlib header when it is present.
            let raw = if packed.len() >= 2
                && packed[0] & 0x0F == 8
                && (packed[0] as u16 * 256 + packed[1] as u16) % 31 == 0
            {
                &packed[2..]
            } else {
                packed
            };
            let mut inflated = inflate(raw).map_err(|_| {
                MdfError::Invalid("compressed data block does not inflate".to_string())
            })?;
            if transposed {
                untranspose(&mut inflated, width);
            }
            out.extend_from_slice(&inflated);
        }
        b"##DL" => {
            // The first link continues the list; the others are the data blocks in order.
            for &link in b.links.iter().skip(1) {
                if link != 0 {
                    data_bytes(input, link, out, depth + 1)?;
                }
            }
            if let Some(&next) = b.links.first() {
                if next != 0 {
                    data_bytes(input, next, out, depth + 1)?;
                }
            }
        }
        _ => return Err(MdfError::Invalid("unexpected block in the data link".to_string())),
    }
    Ok(())
}

/// Undo the transposition of a block: its bytes were stored column by column, with `width` bytes per record. Bytes
/// past the last whole record are left as they are.
fn untranspose(data: &mut Vec<u8>, width: usize) {
    if width == 0 {
        return;
    }
    let rows = data.len() / width;
    let whole = rows * width;
    let mut out = data.clone();
    for c in 0..width {
        for r in 0..rows {
            out[r * width + c] = data[c * rows + r];
        }
    }
    out[whole..].copy_from_slice(&data[whole..]);
    *data = out;
}

/// A channel as stored in its channel group: where its value sits in a record, and how to decode it.
struct ChannelLayout {
    name: String,
    unit: Option<String>,
    kind: u8,
    dtype: u8,
    bit_offset: u8,
    byte_offset: u32,
    bits: u32,
}

/// Read a channel group's channel layouts, following the `CN` chain from the group's first channel.
fn channel_layouts(input: &[u8], first: u64) -> Result<Vec<ChannelLayout>, MdfError> {
    let mut out = Vec::new();
    let mut addr = first;
    let mut guard = 0usize;
    while addr != 0 {
        guard += 1;
        if guard > 1_000_000 {
            return Err(MdfError::Invalid("channel chain does not end".to_string()));
        }
        let cn = block(input, addr)?;
        expect(&cn, b"##CN", "channel")?;
        let d = cn.data;
        let name = text(input, cn.links[2])?;
        let unit = match cn.links.get(6).copied().unwrap_or(0) {
            0 => None,
            u => Some(text(input, u)?),
        };
        out.push(ChannelLayout {
            name,
            unit,
            kind: *d.first().ok_or_else(|| ParseError::eof(addr as usize))?,
            dtype: *d.get(2).ok_or_else(|| ParseError::eof(addr as usize))?,
            bit_offset: *d.get(3).ok_or_else(|| ParseError::eof(addr as usize))?,
            byte_offset: le_u32(d, 4)?,
            bits: le_u32(d, 8)?,
        });
        addr = cn.links[0];
    }
    Ok(out)
}

/// Decode one value from a record, or `None` for types this reader does not decode.
fn decode(record: &[u8], ch: &ChannelLayout) -> Option<f64> {
    let bits = ch.bits as usize;
    if bits == 0 || bits > 64 {
        return None;
    }
    let big_endian = matches!(ch.dtype, 1 | 3 | 5);
    let start = ch.byte_offset as usize;
    let bit_offset = ch.bit_offset as usize;
    let nbytes = (bit_offset + bits).div_ceil(8);
    let bytes = record.get(start..start + nbytes)?;
    let raw = if big_endian {
        if bit_offset != 0 || bits % 8 != 0 {
            return None;
        }
        bytes.iter().fold(0u64, |acc, &b| (acc << 8) | u64::from(b))
    } else {
        let mut v = 0u64;
        for k in 0..bits {
            let bit = bit_offset + k;
            v |= u64::from((bytes[bit / 8] >> (bit % 8)) & 1) << k;
        }
        v
    };
    match ch.dtype {
        0 | 1 => Some(raw as f64),
        2 | 3 => {
            let signed = if bits < 64 && raw & (1u64 << (bits - 1)) != 0 {
                raw | (!0u64 << bits)
            } else {
                raw
            };
            Some(signed as i64 as f64)
        }
        4 | 5 => match bits {
            32 => Some(f32::from_bits(raw as u32) as f64),
            64 => Some(f64::from_bits(raw)),
            _ => None,
        },
        _ => None,
    }
}

/// Read an MDF 4 file.
pub fn parse(input: &[u8]) -> Result<Mdf, MdfError> {
    if input.get(0..8) != Some(&b"MDF     "[..]) {
        return Err(MdfError::Invalid("file does not start with the MDF identifier".to_string()));
    }
    let version = String::from_utf8_lossy(&input[8..16]).trim().to_string();
    let byte_order = u16::from_le_bytes([input[24], input[25]]);
    if byte_order != 0 {
        return Err(MdfError::Unsupported("big-endian MDF files are not read"));
    }
    let header = block(input, 64)?;
    expect(&header, b"##HD", "header")?;
    let mut mdf = Mdf { version, groups: Vec::new() };
    let mut dg = header.links.first().copied().unwrap_or(0);
    while dg != 0 {
        mdf.groups.extend(data_group(input, dg)?);
        let b = block(input, dg)?;
        dg = b.links[0];
    }
    Ok(mdf)
}

/// Read the channel groups of one data group and decode their records.
fn data_group(input: &[u8], addr: u64) -> Result<Vec<ChannelGroup>, MdfError> {
    let dg = block(input, addr)?;
    expect(&dg, b"##DG", "data group")?;
    let record_id_size = *dg.data.first().ok_or_else(|| ParseError::eof(addr as usize))? as usize;

    // Each channel group: its record id, its record length, and its channels.
    struct Group {
        record_id: u64,
        record_len: usize,
        samples: usize,
        name: Option<String>,
        records: u64,
        layouts: Vec<ChannelLayout>,
        values: Vec<Vec<f64>>,
    }
    let mut groups: Vec<Group> = Vec::new();
    let mut cg_addr = dg.links[1];
    while cg_addr != 0 {
        let cg = block(input, cg_addr)?;
        expect(&cg, b"##CG", "channel group")?;
        let d = cg.data;
        let samples = le_u32(d, 24)? as usize;
        let invalidation = le_u32(d, 28)? as usize;
        let name = match cg.links[2] {
            0 => None,
            n => Some(text(input, n)?),
        };
        let layouts = channel_layouts(input, cg.links[1])?;
        let values = alloc::vec![Vec::new(); layouts.len()];
        groups.push(Group {
            record_id: le_u64(d, 0)?,
            record_len: samples + invalidation,
            samples,
            name,
            records: le_u64(d, 8)?,
            layouts,
            values,
        });
        cg_addr = cg.links[0];
    }

    let mut data = Vec::new();
    if dg.links[2] != 0 {
        data_bytes(input, dg.links[2], &mut data, 0)?;
    }
    let mut at = 0usize;
    while at < data.len() {
        let record_id = if record_id_size == 0 {
            0
        } else {
            let mut v = 0u64;
            for k in 0..record_id_size {
                v |= u64::from(*data.get(at + k).ok_or_else(|| ParseError::eof(at))?) << (8 * k);
            }
            at += record_id_size;
            v
        };
        let index = if groups.len() == 1 && record_id_size == 0 {
            0
        } else {
            groups
                .iter()
                .position(|g| g.record_id == record_id)
                .ok_or(MdfError::Invalid("record refers to an unknown channel group".to_string()))?
        };
        let group = &mut groups[index];
        let record = data.get(at..at + group.record_len).ok_or_else(|| ParseError::eof(at))?;
        for (ch, out) in group.layouts.iter().zip(group.values.iter_mut()) {
            if ch.kind == 1 {
                continue;
            }
            if let Some(v) = decode(&record[..group.samples], ch) {
                out.push(v);
            }
        }
        at += group.record_len;
    }

    Ok(groups
        .into_iter()
        .map(|g| ChannelGroup {
            name: g.name,
            records: g.records,
            channels: g
                .layouts
                .into_iter()
                .zip(g.values)
                .filter(|(ch, _)| ch.kind != 1 && ch.bits > 0 && ch.dtype <= 5)
                .map(|(ch, values)| Channel { name: ch.name, unit: ch.unit, values })
                .collect(),
        })
        .collect())
}

/// Find a channel by name in any channel group.
pub fn channel<'a>(mdf: &'a Mdf, name: &str) -> Option<&'a Channel> {
    mdf.groups.iter().flat_map(|g| g.channels.iter()).find(|c| c.name == name)
}
