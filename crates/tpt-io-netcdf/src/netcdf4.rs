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
use std::io::copy;

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
    Chunked { btree_addr: u64, chunk_buf_size: u64 },
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
const MSG_LINKINFO: u16 = 0x0002;
const MSG_DATATYPE: u16 = 0x0003;
const MSG_FILLVALUE: u16 = 0x0005;
const MSG_LINK: u16 = 0x0006;
const MSG_LAYOUT: u16 = 0x0008;
const MSG_FILTER: u16 = 0x000A;
const MSG_FILTER2: u16 = 0x000B;
const MSG_ATTRIBUTE: u16 = 0x000C;
const MSG_OBJHDRCONT: u16 = 0x0010;
const MSG_ATTRINFO: u16 = 0x0015;

fn parse_object_header<'a>(
    r: &Reader<'a>,
    addr: u64,
) -> core::result::Result<(Vec<Message<'a>>, Vec<u64>), NetCdfError> {
    let base = addr as usize;
    if base + 4 > r.data.len() || &r.data[base..base + 4] != b"OHDR" {
        return Err(NetCdfError::Unsupported("expected object header signature"));
    }
    let ver = r.u8(base + 4)?;
    if ver != 2 {
        return Err(NetCdfError::Unsupported("only object header version 2 is supported"))?;
    }
    let mut p = base + 5;
    let flags = r.u8(p)?;
    p += 1;
    let c0w = if flags & 0x01 != 0 { 2 } else { 1 };
    let cnw = if flags & 0x02 != 0 { 2 } else { 1 };
    let chunk0 = r.rd(p, c0w)? as usize;
    p += c0w;
    let _chunkn = r.rd(p, cnw)?;
    p += cnw;
    let nmess = r.rd(p, 2)? as usize;
    p += 2;
    let _hsize = r.rd(p, 4)?;
    p += 4;
    if flags & 0x08 != 0 {
        p += 2;
    }
    if flags & 0x10 != 0 {
        p += 4;
    }
    let mut messages = Vec::new();
    let mut cont = Vec::new();
    let msg_start = p;
    let mut remaining = chunk0.saturating_sub(p - msg_start);
    let mut p = msg_start;
    let mut count = 0usize;
    while count < nmess && p + 6 <= r.data.len() && remaining >= 6 {
        let mtype = r.rd(p, 2)? as u16;
        let msize = r.rd(p + 2, 2)? as usize;
        let mflags = r.u8(p + 4)?;
        p += 6;
        let body = r.slice(p, msize)?;
        let shared = mflags & 0x02 != 0;
        if mtype == MSG_OBJHDRCONT {
            let mut q = 0usize;
            while q + 16 <= body.len() {
                let off = read_int(body, q, 8)?;
                cont.push(off);
                q += 16;
            }
        } else {
            messages.push(Message { mtype, body, shared });
        }
        count += 1;
        p += msize;
        if p % 8 != 0 {
            p += 8 - (p % 8);
        }
        remaining = chunk0.saturating_sub(p - msg_start);
        if remaining == 0 && count < nmess {
            if let Some(next) = cont.pop() {
                p = next as usize;
                if p + 6 > r.data.len() {
                    break;
                }
                // re-check OHDR at continuation; if not OHDR assume continuation data starts directly
                if &r.data[p..p + 4] == b"OHDR" {
                    // small continuation: skip signature and version
                    p += 5;
                }
                remaining = usize::MAX; // continuation chunk is large
            } else {
                break;
            }
        }
    }
    Ok((messages, cont))
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
    r: &Reader<'a>,
    heaps: &[GlobalHeap<'a>],
    body: &'a [u8],
) -> core::result::Result<&'a [u8], NetCdfError> {
    if body.len() < 4 {
        return Err(NetCdfError::Unsupported("shared message too short"));
    }
    let id = u32::from_le_bytes([body[0], body[1], body[2], body[3]]);
    let collection = (id >> 16) as u64;
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
        if let Ok(ds) = parse_dataset(&r, &heaps, link.object_addr) {
            datasets.push(ds);
        }
    }
    if !datasets.iter().any(|d| !d.name.is_empty() && !d.dims.is_empty()) {
        let mut off = sb.root_group_addr as usize + 1;
        while let Some(idx) = r.data[off..].iter().position(|&b| b == b'O') {
            let ohdr = off + idx;
            if ohdr + 4 <= r.data.len() && &r.data[ohdr..ohdr + 4] == b"OHDR" {
                if ohdr != sb.root_group_addr as usize {
                    if let Ok(ds) = parse_dataset(&r, &heaps, ohdr as u64) {
                        if !ds.name.is_empty() || !ds.dims.is_empty() {
                            datasets.push(ds);
                        }
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
    let (msgs, _cont) = parse_object_header(r, addr)?;
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

fn parse_link(_r: &Reader, body: &[u8]) -> core::result::Result<Link, NetCdfError> {
    if body.len() < 8 {
        return Err(NetCdfError::Unsupported("link message too short"));
    }
    let flags = body[1];
    let name_size = u16::from_le_bytes([body[2], body[3]]) as usize;
    let mut p = 4;
    if flags & 0x02 != 0 {
        p += 2;
    }
    let name_bytes =
        body.get(p..p + name_size).ok_or_else(|| NetCdfError::Unsupported("link name oob"))?;
    let name = String::from_utf8_lossy(name_bytes).into_owned();
    p += name_size;
    let object_addr = read_int(body, p, 8)?;
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
    let name_bytes =
        body.get(p..p + name_size).ok_or_else(|| NetCdfError::Unsupported("attr name oob"))?;
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
    let (msgs, _cont) = parse_object_header(r, addr)?;
    let mut name = String::new();
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
                if let Some(d) = parse_dataspace(body) {
                    dims = d;
                }
            }
            MSG_DATATYPE => {
                if let Some(t) = parse_datatype(body) {
                    nc_type = t;
                }
            }
            MSG_LAYOUT => {
                if let Some(l) = parse_layout(r, body) {
                    match &l {
                        Layout::Chunked { chunk_buf_size, .. } => {
                            chunk_dims = Some(parse_chunk_dims(body, *chunk_buf_size));
                        }
                        _ => {}
                    }
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

fn parse_dataspace(body: &[u8]) -> Option<Vec<u64>> {
    if body.len() < 4 {
        return None;
    }
    let ver = body[0];
    let rank = if ver >= 2 { body[3] as usize } else { body[1] as usize };
    let mut p = if ver >= 2 { 4 } else { 2 };
    if ver >= 2 {
        p += 1;
    }
    let mut dims = Vec::new();
    for _ in 0..rank {
        if p + 4 > body.len() {
            break;
        }
        dims.push(u32::from_le_bytes([body[p], body[p + 1], body[p + 2], body[p + 3]]) as u64);
        p += 4;
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

fn parse_layout(r: &Reader, body: &[u8]) -> Option<Layout> {
    if body.is_empty() {
        return None;
    }

    // HDF5 v4 layout messages prefix with a version byte (v2/v3 don't).
    let (version, rest) = if body[0] == 4 { (4, &body[1..]) } else { (body[0], body) };
    if rest.is_empty() {
        return None;
    }
    let class = rest[0];

    match class {
        0 => {
            let size = u32::from_le_bytes([rest[2], rest[3], rest[4], rest[5]]) as usize;
            let data = rest.get(6..6 + size).unwrap_or(&[]).to_vec();
            Some(Layout::Compact { data })
        }
        1 => {
            let ndims = rest[1] as usize;
            let dims_bytes = if version == 4 { ndims * 8 } else { ndims * 4 };
            let p = 2 + dims_bytes;
            let addr = read_int(rest, p, r.offsz).ok()?;
            Some(Layout::Contiguous { addr })
        }
        2 => {
            let ndims = rest[1] as usize;
            let dims_bytes = if version == 4 { ndims * 8 } else { ndims * 4 };
            let mut p = 2 + dims_bytes;
            let chunk_buf_size = if version < 4 && p + 4 <= rest.len() {
                let v = u32::from_le_bytes([rest[p], rest[p + 1], rest[p + 2], rest[p + 3]]) as u64;
                p += 4;
                v
            } else {
                0
            };
            let btree_addr = read_int(rest, p, r.offsz).ok()?;
            Some(Layout::Chunked { btree_addr, chunk_buf_size })
        }
        _ => None,
    }
}

fn parse_chunk_dims(body: &[u8], _buf: u64) -> Vec<u64> {
    if body.len() < 3 {
        return Vec::new();
    }
    // h5py v108 writes v4 layout messages (body[0]=4), skip version byte.
    let body = if body[0] == 4 { &body[1..] } else { body };
    if body.is_empty() || body[0] != 2 {
        return Vec::new();
    }
    let ndims = body[1] as usize;
    let mut p = 2;
    let mut dims = Vec::new();
    let elem = if body.len() >= 3 && body[2] == 0 { 8 } else { 4 };
    for _ in 0..ndims {
        if p + elem > body.len() {
            break;
        }
        if elem == 8 {
            dims.push(u64::from_le_bytes([
                body[p],
                body[p + 1],
                body[p + 2],
                body[p + 3],
                body[p + 4],
                body[p + 5],
                body[p + 6],
                body[p + 7],
            ]));
        } else {
            dims.push(u32::from_le_bytes([body[p], body[p + 1], body[p + 2], body[p + 3]]) as u64);
        }
        p += elem;
    }
    dims
}

fn parse_filters(body: &[u8]) -> Vec<Filter> {
    if body.len() < 6 {
        return Vec::new();
    }
    let nfilters = u16::from_le_bytes([body[2], body[3]]) as usize;
    let mut p = 4;
    let mut out = Vec::new();
    for _ in 0..nfilters {
        if p + 4 > body.len() {
            break;
        }
        let id = u16::from_le_bytes([body[p], body[p + 1]]);
        let _flags = u16::from_le_bytes([body[p + 2], body[p + 3]]);
        p += 4;
        let nparams = u16::from_le_bytes([body[p], body[p + 1]]) as usize;
        p += 2;
        let mut params = Vec::new();
        for _ in 0..nparams {
            if p + 4 > body.len() {
                break;
            }
            params.push(u32::from_le_bytes([body[p], body[p + 1], body[p + 2], body[p + 3]]));
            p += 4;
        }
        out.push(Filter { id, name: Filter::by_id(id), params });
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
            Layout::Chunked { btree_addr, chunk_buf_size: _ } => {
                read_chunked_data(file, self, *btree_addr)
            }
            Layout::Compact { data } => Ok(DecodedData::Raw(data.clone())),
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct ChunkEntry {
    offset: u64,
    size: u64,
    scaled0: u64,
    scaled1: u64,
}

fn read_chunked_data(
    file: &[u8],
    ds: &Dataset,
    btree_addr: u64,
) -> core::result::Result<DecodedData, NetCdfError> {
    let r = Reader { data: file, offsz: 8, lensz: 8 };
    let entries = collect_chunk_entries(&r, btree_addr)?;
    let elem = ds.nc_type.size_of();
    let total = ds.dims.iter().product::<u64>() as usize;
    let mut out = vec![0u8; total * elem];
    let cdims =
        ds.chunk_dims.as_ref().ok_or_else(|| NetCdfError::Unsupported("missing chunk dims"))?;
    if cdims.len() < 2 {
        return Err(NetCdfError::Unsupported("need at least 2D chunks"));
    }
    let nrows = ds.dims[0] as usize;
    let ncols = ds.dims[1] as usize;
    let mut out = vec![0u8; total * elem];
    for e in &entries {
        let raw = file
            .get(e.offset as usize..(e.offset + e.size) as usize)
            .ok_or_else(|| NetCdfError::Core(tpt_io_core::ParseError::eof(e.offset as usize)))?;
        let decoded = decode_chunked_raw(raw, &ds.filters)?;
        // Place decoded chunk into output buffer.
        // Chunk origin in elements: (scaled0, scaled1). Each chunk covers
        // rows [scaled0 .. scaled0+cdims[0]), cols [scaled1 .. scaled1+cdims[1]).
        let c0 = e.scaled0 as usize;
        let c1 = e.scaled1 as usize;
        let cd0 = cdims[0] as usize;
        let cd1 = cdims[1] as usize;
        let nrows = ds.dims[0] as usize;
        let ncols = ds.dims[1] as usize;
        for dr in 0..cd0 {
            let gr = c0 + dr;
            if gr >= nrows {
                break;
            }
            for dc in 0..cd1 {
                let gc = c1 + dc;
                if gc >= ncols {
                    break;
                }
                let dst_off = (gr * ncols + gc) * elem;
                let src_off = (dr * cd1 + dc) * elem;
                if dst_off + elem <= out.len() && src_off + elem <= decoded.len() {
                    out[dst_off..dst_off + elem].copy_from_slice(&decoded[src_off..src_off + elem]);
                }
            }
        }
    }
    Ok(DecodedData::Raw(out))
}

fn collect_chunk_entries(
    r: &Reader<'_>,
    addr: u64,
) -> core::result::Result<Vec<ChunkEntry>, NetCdfError> {
    let base = addr as usize;
    if base + 4 > r.data.len() || &r.data[base..base + 4] != b"TREE" {
        return Err(NetCdfError::Unsupported("expected B-tree signature"));
    }
    let level = r.data[base + 5];
    let used = read_int(r.data, base + 6, 2)? as usize;
    let mut entries = Vec::new();
    if level == 0 {
        let mut p = base + 8 + 2 * r.offsz;
        for _ in 0..used {
            let entry_off = p;
            if entry_off + 40 > r.data.len() {
                break;
            }
            let size = read_int(r.data, entry_off, 8)?;
            let scaled0 = read_int(r.data, entry_off + 8, 8)?;
            let scaled1 = read_int(r.data, entry_off + 16, 8)?;
            let offset = read_int(r.data, entry_off + 32, 8)?;
            entries.push(ChunkEntry { offset, size, scaled0, scaled1 });
            p += 40;
        }
    } else {
        let mut p = base + 8 + 2 * r.offsz;
        for _ in 0..used {
            let child = read_int(r.data, p, r.offsz)?;
            p += r.offsz;
            p += 8 + 8; // skip key
            entries.extend(collect_chunk_entries(r, child)?);
        }
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
                use flate2::read::ZlibDecoder;
                let mut dec = ZlibDecoder::new(&buf[..]);
                let mut out = Vec::new();
                copy(&mut dec, &mut out)
                    .map_err(|_| NetCdfError::Unsupported("deflate decompression failed"))?;
                buf = out;
            }
            _ => {}
        }
    }
    Ok(buf)
}

#[cfg(test)]
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
        let entries = collect_chunk_entries(&r, 0x1800).expect("btree entries");
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
    fn read_chunked4_via_btree() {
        let data = golden("chunked4.h5");
        let ds = Dataset {
            name: "temp".into(),
            dims: vec![4, 6],
            nc_type: NcType::Float,
            layout: Layout::Chunked { btree_addr: 0x1800, chunk_buf_size: 0 },
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
