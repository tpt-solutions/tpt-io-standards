//! NetCDF-4 (HDF5-based) detection, superblock parsing, and object-tree walking.
//!
//! A NetCDF-4 file *is* an HDF5 file. This module descends the HDF5 object
//! tree to enumerate datasets (NetCDF-4 variables), reading their dimensions,
//! element types, storage layout, filter pipeline (deflate/shuffle), and
//! global/per-variable attributes. Contiguous and chunked deflate+shuffle
//! data can both be read back as raw buffers.
//!
//! The implementation targets the on-disk structures produced by `h5py` /
//! `netcdf-c` for the common case (superblock v2/v3 little-endian,
//! object-header v2, contiguous and chunked v1-B-tree storage).

use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

use crate::error::NetCdfError;
use crate::header::NcType;

/// The high-level kind of a NetCDF file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileKind {
    Classic,
    NetCdf4,
}

pub fn detect_format(data: &[u8]) -> Option<FileKind> {
    if data.len() >= 3 && &data[0..3] == b"CDF" {
        Some(FileKind::Classic)
    } else if data.len() >= 8 && &data[0..8] == b"\x89HDF\r\n\x1a\n" {
        Some(FileKind::NetCdf4)
    } else {
        None
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hdf5Superblock {
    pub version: u8,
    pub offset_size: u8,
    pub length_size: u8,
    pub root_group_addr: u64,
}

fn read_int(data: &[u8], pos: usize, size: usize) -> core::result::Result<u64, NetCdfError> {
    let s = data
        .get(pos..pos + size)
        .ok_or_else(|| NetCdfError::Core(tpt_io_core::ParseError::eof(pos)))?;
    let mut v = 0u64;
    for (i, &b) in s.iter().enumerate() {
        v |= (b as u64) << (i * 8);
    }
    Ok(v)
}

pub fn parse_hdf5_superblock(data: &[u8]) -> core::result::Result<Hdf5Superblock, NetCdfError> {
    const SIG: &[u8; 8] = b"\x89HDF\r\n\x1a\n";
    if data.len() < 8 || &data[0..8] != SIG {
        return Err(NetCdfError::Unsupported("not an HDF5/NetCDF-4 file"));
    }
    let version = *data.get(8).ok_or_else(|| NetCdfError::Core(tpt_io_core::ParseError::eof(8)))?;
    let (offset_size, length_size, root_group_addr) = match version {
        1 => {
            let os =
                *data.get(13).ok_or_else(|| NetCdfError::Core(tpt_io_core::ParseError::eof(13)))?;
            let ls =
                *data.get(14).ok_or_else(|| NetCdfError::Core(tpt_io_core::ParseError::eof(14)))?;
            let ra = read_int(data, 24, os as usize)?;
            (os, ls, ra)
        }
        2 | 3 => {
            let os =
                *data.get(9).ok_or_else(|| NetCdfError::Core(tpt_io_core::ParseError::eof(9)))?;
            let ls =
                *data.get(10).ok_or_else(|| NetCdfError::Core(tpt_io_core::ParseError::eof(10)))?;
            let ra = read_int(data, 12 + 3 * (os as usize), os as usize)?;
            (os, ls, ra)
        }
        _ => {
            return Err(NetCdfError::Unsupported(
                "unsupported HDF5 superblock version (only 1/2/3 handled)",
            ));
        }
    };
    Ok(Hdf5Superblock { version, offset_size, length_size, root_group_addr })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct H5Attr {
    pub name: String,
    pub nc_type: NcType,
    pub value: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Filter {
    pub id: u16,
    pub name: &'static str,
    pub params: Vec<u32>,
}

impl Filter {
    fn by_id(id: u16) -> &'static str {
        match id {
            1 => "deflate",
            2 => "shuffle",
            3 => "fletcher32",
            4 => "szip",
            5 => "nbit",
            6 => "scaleoffset",
            _ => "unknown",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Layout {
    Contiguous { addr: u64 },
    Chunked { btree_addr: u64 },
    Compact { data: Vec<u8> },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dataset {
    pub name: String,
    pub dims: Vec<u64>,
    pub nc_type: NcType,
    pub layout: Layout,
    pub chunk_dims: Option<Vec<u64>>,
    pub filters: Vec<Filter>,
    pub attrs: Vec<H5Attr>,
    pub object_addr: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Group {
    pub attrs: Vec<H5Attr>,
    pub links: Vec<Link>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Link {
    pub name: String,
    pub object_addr: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hdf5File {
    pub superblock: Hdf5Superblock,
    pub root: Group,
    pub datasets: Vec<Dataset>,
}

// ---------------------------------------------------------------------------
// Internal object-header / message parsing
// ---------------------------------------------------------------------------

struct Reader<'a> {
    data: &'a [u8],
    offsz: usize,
    lensz: usize,
}

impl<'a> Reader<'a> {
    fn rd(&self, pos: usize, size: usize) -> core::result::Result<u64, NetCdfError> {
        read_int(self.data, pos, size)
    }
    fn u8(&self, pos: usize) -> core::result::Result<u8, NetCdfError> {
        Ok(*self
            .data
            .get(pos)
            .ok_or_else(|| NetCdfError::Core(tpt_io_core::ParseError::eof(pos)))?)
    }
    fn slice(&self, pos: usize, len: usize) -> core::result::Result<&'a [u8], NetCdfError> {
        self.data
            .get(pos..pos + len)
            .ok_or_else(|| NetCdfError::Core(tpt_io_core::ParseError::eof(pos)))
    }
}

struct Message<'a> {
    mtype: u16,
    body: &'a [u8],
    shared: bool,
}

const MSG_DATASPACE: u16 = 0x0001;
const MSG_DATATYPE: u16 = 0x0003;
const MSG_LINK: u16 = 0x0006;
const MSG_LAYOUT: u16 = 0x0008;
const MSG_FILTER: u16 = 0x000A;
const MSG_FILTER2: u16 = 0x000B;
const MSG_ATTRIBUTE: u16 = 0x000C;
const MSG_OBJHDRCONT: u16 = 0x0010;

/// Read the messages of a version-2 object header.
///
/// The header is `OHDR`, version, flags, optional timestamps and attribute
/// counts, then the size of chunk 0 and the messages. Message headers are
/// type (1 byte), size (2 bytes), flags (1 byte), plus a 2-byte creation order
/// when the object tracks it. Messages are not padded. Each chunk is followed
/// by a 4-byte checksum that is not part of its size. Continuation messages
/// point at `OCHK` blocks whose messages are read the same way.
fn parse_object_header<'a>(
    r: &Reader<'a>,
    addr: u64,
) -> core::result::Result<Vec<Message<'a>>, NetCdfError> {
    let base = addr as usize;
    if r.slice(base, 4)? != &b"OHDR"[..] {
        return Err(NetCdfError::Unsupported("expected object header signature"));
    }
    if r.u8(base + 4)? != 2 {
        return Err(NetCdfError::Unsupported("only object header version 2 is supported"));
    }
    let flags = r.u8(base + 5)?;
    let mut p = base + 6;
    if flags & 0x20 != 0 {
        // Access, modification, change and birth times (4 bytes each).
        p += 16;
    }
    if flags & 0x10 != 0 {
        // Maximum compact and minimum dense attribute counts (2 bytes each).
        p += 4;
    }
    let size_width = 1usize << (flags & 0x03);
    let chunk0_len = r.rd(p, size_width)? as usize;
    p += size_width;
    let track_order = flags & 0x04 != 0;

    let mut messages = Vec::new();
    let mut continuations: Vec<(usize, usize)> = Vec::new();
    read_messages(r, p, p + chunk0_len, track_order, &mut messages, &mut continuations)?;

    let mut i = 0;
    while i < continuations.len() {
        let (off, len) = continuations[i];
        i += 1;
        if len < 8 || r.slice(off, 4)? != &b"OCHK"[..] {
            return Err(NetCdfError::Unsupported("bad object header continuation block"));
        }
        // Skip the signature at the start and the checksum at the end.
        read_messages(
            r,
            off + 4,
            off + len - 4,
            track_order,
            &mut messages,
            &mut continuations,
        )?;
    }
    Ok(messages)
}

/// Read version-2 object header messages in `[start, end)`.
fn read_messages<'a>(
    r: &Reader<'a>,
    start: usize,
    end: usize,
    track_order: bool,
    out: &mut Vec<Message<'a>>,
    continuations: &mut Vec<(usize, usize)>,
) -> core::result::Result<(), NetCdfError> {
    let header_len = if track_order { 6 } else { 4 };
    let mut p = start;
    while p + header_len <= end {
        let mtype = r.u8(p)? as u16;
        let msize = r.rd(p + 1, 2)? as usize;
        let mflags = r.u8(p + 3)?;
        let body = r.slice(p + header_len, msize)?;
        match mtype {
            0 => {}
            MSG_OBJHDRCONT => {
                let off = read_int(body, 0, r.offsz)? as usize;
                let len = read_int(body, r.offsz, r.lensz)? as usize;
                continuations.push((off, len));
            }
            _ => out.push(Message { mtype, body, shared: mflags & 0x02 != 0 }),
        }
        p += header_len + msize;
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Global heap (shared message storage)
// ---------------------------------------------------------------------------

struct GlobalHeap<'a> {
    objects: Vec<(u64, &'a [u8])>,
}

fn parse_global_heaps<'a>(r: &Reader<'a>) -> Vec<GlobalHeap<'a>> {
    let mut heaps = Vec::new();
    let mut start = 0usize;
    while let Some(pos) = r.data[start..].iter().position(|&b| b == b'G') {
        let idx = start + pos;
        if idx + 4 <= r.data.len() && &r.data[idx..idx + 4] == b"GCOL" {
            let mut p = idx + 8;
            p += r.lensz; // collection size / reserved field
            let mut objects = Vec::new();
            while p + 8 <= r.data.len() {
                let oid = r.rd(p, 2).unwrap_or(0);
                let _ref = r.rd(p + 2, 2).unwrap_or(0);
                let size = r.rd(p + 4, r.lensz).unwrap_or(0) as usize;
                p += 2 + 2 + r.lensz;
                if size == 0 {
                    break;
                }
                let data = match r.slice(p, size) {
                    Ok(s) => s,
                    Err(_) => break,
                };
                objects.push((oid, data));
                p += size;
                if p % 8 != 0 {
                    p += 8 - (p % 8);
                }
            }
            heaps.push(GlobalHeap { objects });
        }
        start = idx + 4;
    }
    heaps
}

fn resolve_shared<'a>(
    _r: &Reader<'a>,
    heaps: &[GlobalHeap<'a>],
    body: &'a [u8],
) -> core::result::Result<&'a [u8], NetCdfError> {
    if body.len() < 4 {
        return Err(NetCdfError::Unsupported("shared message too short"));
    }
    let id = u32::from_le_bytes([body[0], body[1], body[2], body[3]]);
    let _collection = (id >> 16) as u64;
    let obj = (id & 0xFFFF) as u64;
    for h in heaps {
        if let Some((_, d)) = h.objects.iter().find(|(o, _)| *o == obj) {
            return Ok(d);
        }
    }
    Err(NetCdfError::Unsupported("shared message object not found"))
}

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

pub fn parse(data: &[u8]) -> core::result::Result<Hdf5File, NetCdfError> {
    let sb = parse_hdf5_superblock(data)?;
    let r = Reader { data, offsz: sb.offset_size as usize, lensz: sb.length_size as usize };
    let heaps = parse_global_heaps(&r);
    let root = parse_group(&r, &heaps, sb.root_group_addr)?;
    let mut datasets = Vec::new();
    for link in &root.links {
        if let Ok(mut ds) = parse_dataset(&r, &heaps, link.object_addr) {
            ds.name = link.name.clone();
            datasets.push(ds);
        }
    }
    if !datasets.iter().any(|d| !d.name.is_empty() && !d.dims.is_empty()) {
        let mut off = sb.root_group_addr as usize + 1;
        while let Some(idx) = r.data[off..].iter().position(|&b| b == b'O') {
            let ohdr = off + idx;
            if ohdr + 4 <= r.data.len()
                && &r.data[ohdr..ohdr + 4] == b"OHDR"
                && ohdr != sb.root_group_addr as usize
            {
                if let Ok(ds) = parse_dataset(&r, &heaps, ohdr as u64) {
                    if !ds.name.is_empty() || !ds.dims.is_empty() {
                        datasets.push(ds);
                    }
                }
            }
            off = ohdr + 4;
            if off >= r.data.len() {
                break;
            }
        }
    }
    Ok(Hdf5File { superblock: sb, root, datasets })
}

fn parse_group(
    r: &Reader,
    heaps: &[GlobalHeap],
    addr: u64,
) -> core::result::Result<Group, NetCdfError> {
    let msgs = parse_object_header(r, addr)?;
    let mut attrs = Vec::new();
    let mut links = Vec::new();
    for m in &msgs {
        let body = if m.shared {
            match resolve_shared(r, heaps, m.body) {
                Ok(b) => b,
                Err(_) => continue,
            }
        } else {
            m.body
        };
        match m.mtype {
            MSG_LINK => {
                if let Ok(l) = parse_link(r, body) {
                    links.push(l);
                }
            }
            MSG_ATTRIBUTE => {
                if let Ok(a) = parse_attribute(r, body) {
                    attrs.push(a);
                }
            }
            _ => {}
        }
    }
    Ok(Group { attrs, links })
}

/// Parse a version-1 link message. Only hard links carry an object address;
/// soft and external links are reported as unsupported and skipped.
fn parse_link(r: &Reader, body: &[u8]) -> core::result::Result<Link, NetCdfError> {
    if body.len() < 2 || body[0] != 1 {
        return Err(NetCdfError::Unsupported("unsupported link message"));
    }
    let flags = body[1];
    let mut p = 2;
    let mut link_type = 0u8;
    if flags & 0x08 != 0 {
        link_type = *body.get(p).ok_or(NetCdfError::Unsupported("link type oob"))?;
        p += 1;
    }
    if flags & 0x04 != 0 {
        // Creation order (8 bytes).
        p += 8;
    }
    if flags & 0x10 != 0 {
        // Character set (1 byte).
        p += 1;
    }
    let name_width = 1usize << (flags & 0x03);
    let name_len = read_int(body, p, name_width)? as usize;
    p += name_width;
    let name_bytes = body.get(p..p + name_len).ok_or(NetCdfError::Unsupported("link name oob"))?;
    let name = String::from_utf8_lossy(name_bytes).into_owned();
    p += name_len;
    if link_type != 0 {
        return Err(NetCdfError::Unsupported("only hard links are followed"));
    }
    let object_addr = read_int(body, p, r.offsz)?;
    Ok(Link { name, object_addr })
}

fn parse_attribute(r: &Reader, body: &[u8]) -> core::result::Result<H5Attr, NetCdfError> {
    if body.len() < 8 {
        return Err(NetCdfError::Unsupported("attribute message too short"));
    }
    let name_size = u16::from_le_bytes([body[1], body[2]]) as usize;
    let dtype_size = u16::from_le_bytes([body[3], body[4]]) as usize;
    let dspace_size = u16::from_le_bytes([body[5], body[6]]) as usize;
    let mut p = 7;
    let name_bytes = body.get(p..p + name_size).ok_or(NetCdfError::Unsupported("attr name oob"))?;
    let name = String::from_utf8_lossy(name_bytes).into_owned();
    p += name_size;
    let _dt = r.slice(p, dtype_size)?;
    p += dtype_size;
    p += dspace_size;
    let value = body.get(p..).unwrap_or(&[]).to_vec();
    let nc_type = match dtype_size {
        1 => NcType::Ubyte,
        2 => NcType::Short,
        4 => NcType::Int,
        8 => NcType::Double,
        _ => NcType::Float,
    };
    Ok(H5Attr { name, nc_type, value })
}

fn parse_dataset(
    r: &Reader,
    heaps: &[GlobalHeap],
    addr: u64,
) -> core::result::Result<Dataset, NetCdfError> {
    let msgs = parse_object_header(r, addr)?;
    let name = String::new();
    let mut dims: Vec<u64> = Vec::new();
    let mut nc_type = NcType::Float;
    let mut layout = Layout::Contiguous { addr: 0 };
    let mut chunk_dims: Option<Vec<u64>> = None;
    let mut filters: Vec<Filter> = Vec::new();
    let mut attrs: Vec<H5Attr> = Vec::new();
    for m in &msgs {
        let body = if m.shared {
            match resolve_shared(r, heaps, m.body) {
                Ok(b) => b,
                Err(_) => continue,
            }
        } else {
            m.body
        };
        match m.mtype {
            MSG_DATASPACE => {
                if let Some(d) = parse_dataspace(body, r.lensz) {
                    dims = d;
                }
            }
            MSG_DATATYPE => {
                if let Some(t) = parse_datatype(body) {
                    nc_type = t;
                }
            }
            MSG_LAYOUT => {
                if let Some((l, dims)) = parse_layout(r, body) {
                    chunk_dims = dims;
                    layout = l;
                }
            }
            MSG_FILTER | MSG_FILTER2 => {
                for f in parse_filters(body) {
                    filters.push(f);
                }
            }
            MSG_ATTRIBUTE => {
                if let Ok(a) = parse_attribute(r, body) {
                    attrs.push(a);
                }
            }
            _ => {}
        }
    }
    Ok(Dataset { name, dims, nc_type, layout, chunk_dims, filters, attrs, object_addr: addr })
}

/// Parse a dataspace message (versions 1 and 2) into its dimension sizes.
///
/// Layout: version, rank, flags, then version 1 has 5 reserved bytes and
/// version 2 has a type byte before the dimensions. Each dimension is
/// `lensz` bytes. Maximum dimensions, when present, are ignored.
fn parse_dataspace(body: &[u8], lensz: usize) -> Option<Vec<u64>> {
    let rank = *body.get(1)? as usize;
    let mut p = match *body.first()? {
        1 => 8,
        2 => 4,
        _ => return None,
    };
    let mut dims = Vec::with_capacity(rank);
    for _ in 0..rank {
        dims.push(read_int(body, p, lensz).ok()?);
        p += lensz;
    }
    Some(dims)
}

fn parse_datatype(body: &[u8]) -> Option<NcType> {
    if body.len() < 4 {
        return None;
    }
    let class = body[1];
    let size = u16::from_le_bytes([body[2], body[3]]) as usize;
    match class {
        0 => match size {
            1 => Some(NcType::Ubyte),
            2 => Some(NcType::Short),
            4 => Some(NcType::Int),
            _ => Some(NcType::Int),
        },
        1 => match size {
            4 => Some(NcType::Float),
            8 => Some(NcType::Double),
            _ => Some(NcType::Float),
        },
        _ => Some(NcType::Float),
    }
}

/// Parse a version-3 data layout message.
///
/// Layout: version (3), class, then per class: compact stores a 2-byte size
/// and the data; contiguous stores the address; chunked stores the rank plus
/// one, the B-tree address and the chunk dimensions as 4-byte values. The last
/// chunk dimension is the element size and is not part of the chunk shape.
/// Returns the layout and, for chunked storage, the chunk dimensions.
fn parse_layout(r: &Reader, body: &[u8]) -> Option<(Layout, Option<Vec<u64>>)> {
    if body.len() < 2 || body[0] != 3 {
        return None;
    }
    match body[1] {
        0 => {
            let size = read_int(body, 2, 2).ok()? as usize;
            let data = body.get(4..4 + size)?.to_vec();
            Some((Layout::Compact { data }, None))
        }
        1 => {
            let addr = read_int(body, 2, r.offsz).ok()?;
            Some((Layout::Contiguous { addr }, None))
        }
        2 => {
            let dimensionality = *body.get(2)? as usize;
            let btree_addr = read_int(body, 3, r.offsz).ok()?;
            let mut p = 3 + r.offsz;
            let mut dims = Vec::with_capacity(dimensionality);
            for _ in 0..dimensionality {
                dims.push(read_int(body, p, 4).ok()?);
                p += 4;
            }
            dims.pop();
            Some((Layout::Chunked { btree_addr }, Some(dims)))
        }
        _ => None,
    }
}

/// Parse a filter pipeline message (versions 1 and 2), in application order.
///
/// Layout: version, filter count, then per filter an id, an optional name
/// length, flags, a parameter count, the name, and the parameters. Version 1
/// pads odd parameter counts to 8 bytes, and it always carries a name length.
/// Version 2 carries a name length only for filter ids of 256 or more, and it
/// does not pad.
fn parse_filters(body: &[u8]) -> Vec<Filter> {
    let rd = |pos: usize, n: usize| read_int(body, pos, n).ok();
    let (Some(&version), Some(&count)) = (body.first(), body.get(1)) else {
        return Vec::new();
    };
    let mut p = match version {
        1 => 8,
        2 => 2,
        _ => return Vec::new(),
    };
    let mut out = Vec::new();
    for _ in 0..count {
        let Some(id) = rd(p, 2) else { break };
        p += 2;
        let name_len = if version == 1 || id >= 256 {
            let Some(n) = rd(p, 2) else { break };
            p += 2;
            n as usize
        } else {
            0
        };
        let Some(_flags) = rd(p, 2) else { break };
        p += 2;
        let Some(nparams) = rd(p, 2) else { break };
        p += 2;
        // A version-1 name length already includes its 8-byte padding.
        p += name_len;
        let mut params = Vec::new();
        for _ in 0..nparams {
            let Some(v) = rd(p, 4) else { break };
            params.push(v as u32);
            p += 4;
        }
        if version == 1 && nparams % 2 == 1 {
            p += 4;
        }
        out.push(Filter { id: id as u16, name: Filter::by_id(id as u16), params });
    }
    out
}

// ---------------------------------------------------------------------------
// Data reading helpers
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub enum DecodedData {
    Raw(Vec<u8>),
    F64(Vec<f64>),
}

impl DecodedData {
    pub fn try_raw(self) -> Option<Vec<u8>> {
        match self {
            DecodedData::Raw(b) => Some(b),
            _ => None,
        }
    }
}

impl Dataset {
    pub fn read_data(&self, file: &[u8]) -> core::result::Result<DecodedData, NetCdfError> {
        match &self.layout {
            Layout::Contiguous { addr } => {
                let start = *addr as usize;
                let len = self.dims.iter().product::<u64>() as usize * self.nc_type.size_of();
                let bytes = file
                    .get(start..start + len)
                    .ok_or_else(|| NetCdfError::Core(tpt_io_core::ParseError::eof(start)))?;
                Ok(DecodedData::Raw(bytes.to_vec()))
            }
            Layout::Chunked { btree_addr } => {
                read_chunked_data(file, self, *btree_addr)
            }
            Layout::Compact { data } => Ok(DecodedData::Raw(data.clone())),
        }
    }
}

/// One chunk of a chunked dataset: where its (possibly filtered) bytes live,
/// and the element offset of its origin in each dimension.
#[derive(Debug, Clone)]
struct ChunkEntry {
    offset: u64,
    size: u64,
    origin: Vec<u64>,
}

/// Assemble a chunked dataset into a contiguous row-major buffer.
///
/// Each chunk is decoded through its filter pipeline and copied into place by
/// its N-dimensional origin. Chunks at the high edges may be partly outside the
/// dataset; those elements are dropped.
fn read_chunked_data(
    file: &[u8],
    ds: &Dataset,
    btree_addr: u64,
) -> core::result::Result<DecodedData, NetCdfError> {
    let r = Reader { data: file, offsz: 8, lensz: 8 };
    let rank = ds.dims.len();
    let cdims = ds.chunk_dims.as_ref().ok_or(NetCdfError::Unsupported("missing chunk dims"))?;
    if rank == 0 || cdims.len() != rank {
        return Err(NetCdfError::Unsupported("chunk rank does not match dataspace"));
    }
    let entries = collect_chunk_entries(&r, btree_addr, rank)?;
    let elem = ds.nc_type.size_of();
    let shape: Vec<usize> = ds.dims.iter().map(|&d| d as usize).collect();
    let chunk_shape: Vec<usize> = cdims.iter().map(|&d| d as usize).collect();
    let chunk_elems: usize = chunk_shape.iter().product();
    let total: usize = shape.iter().product();

    // Row-major element strides of the full dataset.
    let mut stride = vec![1usize; rank];
    for i in (0..rank - 1).rev() {
        stride[i] = stride[i + 1] * shape[i + 1];
    }

    let mut out = vec![0u8; total * elem];
    for e in &entries {
        let raw = file
            .get(e.offset as usize..(e.offset + e.size) as usize)
            .ok_or_else(|| NetCdfError::Core(tpt_io_core::ParseError::eof(e.offset as usize)))?;
        let decoded = decode_chunked_raw(raw, &ds.filters)?;
        if decoded.len() < chunk_elems * elem {
            return Err(NetCdfError::Unsupported("decoded chunk is shorter than its shape"));
        }
        for lin in 0..chunk_elems {
            // Unravel the chunk-local index (row-major) and map it to the dataset.
            let mut rem = lin;
            let mut dst = 0usize;
            let mut inside = true;
            for i in (0..rank).rev() {
                let local = rem % chunk_shape[i];
                rem /= chunk_shape[i];
                let g = e.origin[i] as usize + local;
                if g >= shape[i] {
                    inside = false;
                    break;
                }
                dst += g * stride[i];
            }
            if inside {
                out[dst * elem..(dst + 1) * elem]
                    .copy_from_slice(&decoded[lin * elem..(lin + 1) * elem]);
            }
        }
    }
    Ok(DecodedData::Raw(out))
}

/// Walk a version-1 chunk B-tree and collect its leaf entries.
///
/// A node is `TREE`, type, level, entry count, two sibling addresses, then
/// `key0, child0, key1, child1, …`. A chunk key is a 4-byte size, a 4-byte
/// filter mask, and `rank + 1` 8-byte element offsets (the last is the element
/// dimension). The child is the chunk's file address at leaves, or the next
/// node below an inner level.
fn collect_chunk_entries(
    r: &Reader<'_>,
    addr: u64,
    rank: usize,
) -> core::result::Result<Vec<ChunkEntry>, NetCdfError> {
    let base = addr as usize;
    if r.slice(base, 4)? != &b"TREE"[..] {
        return Err(NetCdfError::Unsupported("expected B-tree signature"));
    }
    let level = r.u8(base + 5)?;
    let used = r.rd(base + 6, 2)? as usize;
    let key_len = 8 + 8 * (rank + 1);
    let mut entries = Vec::new();
    let mut p = base + 8 + 2 * r.offsz;
    for _ in 0..used {
        let size = r.rd(p, 4)?;
        let mut origin = Vec::with_capacity(rank);
        for i in 0..rank {
            origin.push(r.rd(p + 8 + 8 * i, 8)?);
        }
        let child = r.rd(p + key_len, r.offsz)?;
        if level == 0 {
            entries.push(ChunkEntry { offset: child, size, origin });
        } else {
            entries.extend(collect_chunk_entries(r, child, rank)?);
        }
        p += key_len + r.offsz;
    }
    Ok(entries)
}

/// Unshuffle a buffer that was shuffled by the HDF5 shuffle filter.
///
/// The shuffle filter rearranges bytes so that byte k of all elements are
/// stored contiguously. `elem_size` is the size of one element in bytes
/// (e.g. 4 for f32, 8 for f64). The input length must be a multiple of
/// `elem_size`.
fn unshuffle(buf: &[u8], elem_size: usize) -> Vec<u8> {
    let nelems = buf.len() / elem_size;
    let mut out = vec![0u8; buf.len()];
    for k in 0..elem_size {
        for i in 0..nelems {
            out[i * elem_size + k] = buf[k * nelems + i];
        }
    }
    out
}

/// Inflate a zlib (deflate) stream.
#[cfg(feature = "std")]
fn inflate(buf: &[u8]) -> Result<Vec<u8>, NetCdfError> {
    use std::io::copy;

    let mut dec = flate2::read::ZlibDecoder::new(buf);
    let mut out = Vec::new();
    copy(&mut dec, &mut out)
        .map_err(|_| NetCdfError::Unsupported("deflate decompression failed"))?;
    Ok(out)
}

/// Without `std` there is no deflate backend, so compressed chunks are
/// reported as unsupported rather than failing to build.
#[cfg(not(feature = "std"))]
fn inflate(_buf: &[u8]) -> Result<Vec<u8>, NetCdfError> {
    Err(NetCdfError::Unsupported("deflate requires the `std` feature"))
}

/// Decode a raw chunk buffer that may be wrapped in a zlib (deflate) stream
/// and preceded by a shuffle filter.
///
/// Filters are stored in the order they were applied during encoding.
/// Decoding must apply them in reverse: deflate(inflate) first, then unshuffle.
pub fn decode_chunked_raw(
    compressed: &[u8],
    filters: &[Filter],
) -> core::result::Result<Vec<u8>, NetCdfError> {
    let mut buf = compressed.to_vec();
    for f in filters.iter().rev() {
        match f.id {
            2 => {
                let elem = f.params.first().copied().unwrap_or(4) as usize;
                if elem > 0 && buf.len() % elem == 0 {
                    buf = unshuffle(&buf, elem);
                }
            }
            1 => {
                buf = inflate(&buf)?;
            }
            _ => {}
        }
    }
    Ok(buf)
}

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    fn golden(name: &str) -> Vec<u8> {
        let mut p = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        p.push("tests");
        p.push("samples");
        p.push(name);
        std::fs::read(&p).unwrap()
    }

    #[test]
    fn detects() {
        assert_eq!(
            detect_format(&[0x89, 0x48, 0x44, 0x46, 0x0d, 0x0a, 0x1a, 0x0a]),
            Some(FileKind::NetCdf4)
        );
    }

    #[test]
    fn parse_example4_metadata() {
        let data = golden("example4.h5");
        let f = parse(&data).expect("parse example4.h5");
        let sb = &f.superblock;
        assert_eq!(sb.version, 2);
        assert_eq!(sb.root_group_addr, 48);
        assert!(detect_format(&data) == Some(FileKind::NetCdf4));
    }

    #[test]
    fn parse_chunked4_metadata() {
        let data = golden("chunked4.h5");
        let _f = parse(&data).expect("parse chunked4.h5");
        let r = Reader { data: &data, offsz: 8, lensz: 8 };
        let entries = collect_chunk_entries(&r, 0x1800, 2).expect("btree entries");
        assert_eq!(entries.len(), 4, "expected 4 chunks, got {}", entries.len());
        assert!(entries.iter().any(|e| e.offset == 8760), "expected chunk at offset 8760");
    }

    #[test]
    fn read_contiguous_example4() {
        let data = golden("example4.h5");
        let ds = Dataset {
            name: "time".into(),
            dims: vec![2],
            nc_type: NcType::Float,
            layout: Layout::Contiguous { addr: 15568 },
            chunk_dims: None,
            filters: vec![],
            attrs: vec![],
            object_addr: 0,
        };
        let raw = ds.read_data(&data).expect("read contiguous").try_raw().expect("raw");
        let mut vals = Vec::new();
        for chunk in raw.chunks(4) {
            if chunk.len() == 4 {
                vals.push(f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]) as f64);
            }
        }
        assert_eq!(vals, vec![1.0, 2.0]);
    }

    #[test]
    fn decode_chunked_raw_chunks() {
        let filters = vec![
            Filter { id: 2, name: "shuffle", params: vec![4] },
            Filter { id: 1, name: "deflate", params: vec![] },
        ];
        let chunks: &[(u64, &[u8])] = &[
            (8760, &include_bytes!("../tests/samples/chunks/chunk_0.raw")[..]),
            (8782, &include_bytes!("../tests/samples/chunks/chunk_1.raw")[..]),
            (8805, &include_bytes!("../tests/samples/chunks/chunk_2.raw")[..]),
            (8826, &include_bytes!("../tests/samples/chunks/chunk_3.raw")[..]),
        ];
        let elem = 4;
        for (_, raw) in chunks {
            let decoded = decode_chunked_raw(raw, &filters).expect("decode chunk");
            assert!(
                decoded.len() % elem == 0,
                "decoded len {} not multiple of elem {}",
                decoded.len(),
                elem
            );
            assert!(decoded.len() <= 24, "decoded chunk too large: {}", decoded.len());
        }
    }

    #[test]
    fn parse_and_read_chunked4_end_to_end() {
        // Metadata, layout and filters all come from the file, not from fixtures.
        let bytes = golden("chunked4.h5");
        let file = parse(&bytes).expect("parse chunked4.h5");
        let ds = file.datasets.iter().find(|d| d.name == "temp").expect("temp");
        assert_eq!(ds.dims, vec![4, 6]);
        assert_eq!(ds.chunk_dims.as_deref(), Some(&[2, 3][..]));
        let names: Vec<&str> = ds.filters.iter().map(|f| f.name).collect();
        assert_eq!(names, ["shuffle", "deflate"]);

        let decoded = ds.read_data(&bytes).expect("read").try_raw().expect("raw");
        for i in 0..4 {
            for j in 0..6 {
                let off = (i * 6 + j) * 4;
                let v = f32::from_le_bytes(decoded[off..off + 4].try_into().unwrap());
                assert_eq!(v, (i * 10 + j) as f32, "temp[{i}][{j}]");
            }
        }
    }

    #[test]
    fn read_chunked_4d_places_every_element() {
        // 2x3x4x5 variable, chunks 1x2x3x2 (partial edge chunks), shuffle + deflate.
        let bytes = golden("chunked4d.h5");
        let file = parse(&bytes).expect("parse chunked4d.h5");
        let ds = file.datasets.iter().find(|d| d.name == "temp4d").expect("temp4d");
        assert_eq!(ds.dims, vec![2, 3, 4, 5]);
        assert_eq!(ds.chunk_dims.as_deref(), Some(&[1, 2, 3, 2][..]));

        let decoded = ds.read_data(&bytes).expect("read").try_raw().expect("raw");
        let total = 2 * 3 * 4 * 5;
        assert_eq!(decoded.len(), total * 4);
        for k in 0..total {
            let v = f32::from_le_bytes(decoded[k * 4..k * 4 + 4].try_into().unwrap());
            assert_eq!(v, k as f32, "element {k} holds {v}");
        }
    }

    #[test]
    fn read_chunked4_via_btree() {
        let data = golden("chunked4.h5");
        let ds = Dataset {
            name: "temp".into(),
            dims: vec![4, 6],
            nc_type: NcType::Float,
            layout: Layout::Chunked { btree_addr: 0x1800 },
            chunk_dims: Some(vec![2, 3]),
            filters: vec![
                Filter { id: 2, name: "shuffle", params: vec![4] },
                Filter { id: 1, name: "deflate", params: vec![] },
            ],
            attrs: vec![],
            object_addr: 0,
        };
        let decoded = ds.read_data(&data).expect("read chunked").try_raw().expect("raw");
        assert_eq!(decoded.len(), 4 * 6 * 4, "expected 96 bytes, got {}", decoded.len());
        for i in 0..4 {
            for j in 0..6 {
                let off = (i * 6 + j) * 4;
                let v = f32::from_le_bytes([
                    decoded[off],
                    decoded[off + 1],
                    decoded[off + 2],
                    decoded[off + 3],
                ]);
                assert_eq!(
                    v,
                    (i * 10 + j) as f32,
                    "temp[{}][{}] = {}, expected {}",
                    i,
                    j,
                    v,
                    i * 10 + j
                );
            }
        }
    }
}
