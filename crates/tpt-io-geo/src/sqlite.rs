//! A read-only SQLite 3 reader: the file header, table B-trees, overflow pages and record decoding.
//!
//! It reads tables in rowid order, which is what GeoPackage feature tables need. Indexes, the write-ahead log,
//! `WITHOUT ROWID` tables and UTF-16 text are not read. The file format is described at sqlite.org (fileformat2).

use alloc::string::{String, ToString};
use alloc::vec::Vec;

use crate::error::GeoError;

const HEADER_SIZE: usize = 100;
const INT_SIZES: [usize; 7] = [0, 1, 2, 3, 4, 6, 8];

/// A decoded column value.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    /// SQL NULL.
    Null,
    /// A signed integer.
    Int(i64),
    /// A floating-point number.
    Real(f64),
    /// UTF-8 text.
    Text(String),
    /// A binary blob.
    Blob(Vec<u8>),
}

/// One row of a table: its rowid and its column values in table order.
#[derive(Debug, Clone, PartialEq)]
pub struct Row {
    /// The rowid. An `INTEGER PRIMARY KEY` column is stored as NULL and is the rowid.
    pub rowid: i64,
    /// The column values, in the order the table defines them.
    pub values: Vec<Value>,
}

/// One entry of the schema table (`sqlite_master`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Schema {
    /// `table`, `index`, `view` or `trigger`.
    pub kind: String,
    /// The object name.
    pub name: String,
    /// The table the object belongs to.
    pub table: String,
    /// The root page of the object's B-tree.
    pub root_page: u32,
    /// The `CREATE` statement that defines the object.
    pub sql: String,
}

/// A SQLite database file, borrowing its bytes.
#[derive(Debug, Clone)]
pub struct Database<'a> {
    data: &'a [u8],
    page_size: usize,
    usable: usize,
}

impl<'a> Database<'a> {
    /// Read the database header.
    pub fn open(data: &'a [u8]) -> Result<Self, GeoError> {
        if data.len() < HEADER_SIZE || !data.starts_with(b"SQLite format 3\0") {
            return Err(GeoError::custom(0, "not an SQLite 3 database"));
        }
        let raw = u16_at(data, 16).ok_or_else(|| GeoError::eof(16))?;
        let page_size = if raw == 1 { 65536 } else { raw as usize };
        if !(512..=65536).contains(&page_size) || !page_size.is_power_of_two() {
            return Err(GeoError::custom(16, "invalid SQLite page size"));
        }
        let encoding = u32_at(data, 56).ok_or_else(|| GeoError::eof(56))?;
        if encoding != 1 {
            return Err(GeoError::Unsupported("only UTF-8 SQLite text is read"));
        }
        let reserved = *data.get(20).ok_or_else(|| GeoError::eof(20))? as usize;
        let usable = page_size.checked_sub(reserved).filter(|&u| u >= 480);
        let usable = usable
            .ok_or_else(|| GeoError::custom(20, "SQLite reserved space leaves too little page"))?;
        Ok(Database { data, page_size, usable })
    }

    /// The schema table: every table, index, view and trigger in the database.
    pub fn schema(&self) -> Result<Vec<Schema>, GeoError> {
        let mut out = Vec::new();
        for row in self.rows(1)? {
            let v = &row.values;
            if v.len() < 5 {
                continue;
            }
            out.push(Schema {
                kind: text(&v[0]),
                name: text(&v[1]),
                table: text(&v[2]),
                root_page: int(&v[3]).unwrap_or(0) as u32,
                sql: text(&v[4]),
            });
        }
        Ok(out)
    }

    /// The root page of the table called `name`.
    pub fn table_root(&self, name: &str) -> Result<u32, GeoError> {
        self.schema()?
            .iter()
            .find(|s| s.kind == "table" && s.name == name)
            .map(|s| s.root_page)
            .ok_or_else(|| GeoError::custom(0, alloc::format!("no table named {name}")))
    }

    /// Every row of the table rooted at `page`, in rowid order.
    pub fn rows(&self, page: u32) -> Result<Vec<Row>, GeoError> {
        let mut out = Vec::new();
        self.collect(page, &mut out, 0)?;
        Ok(out)
    }

    fn page_base(&self, page: u32) -> Result<usize, GeoError> {
        if page == 0 {
            return Err(GeoError::custom(0, "page number 0 is not a page"));
        }
        let base = (page as usize - 1) * self.page_size;
        if base + self.page_size > self.data.len() {
            return Err(GeoError::eof(base));
        }
        Ok(base)
    }

    fn collect(&self, page: u32, out: &mut Vec<Row>, depth: usize) -> Result<(), GeoError> {
        if depth > 64 {
            return Err(GeoError::custom(0, "table B-tree is nested too deeply"));
        }
        let base = self.page_base(page)?;
        // Page 1 keeps its B-tree header after the 100-byte database header.
        let head = if page == 1 { base + HEADER_SIZE } else { base };
        let kind = self.byte(head)?;
        let cells = self.u16(head + 3)? as usize;
        match kind {
            5 => {
                for i in 0..cells {
                    let ptr = self.u16(head + 12 + 2 * i)? as usize;
                    let child = self.u32(base + ptr)?;
                    self.collect(child, out, depth + 1)?;
                }
                let right = self.u32(head + 8)?;
                self.collect(right, out, depth + 1)?;
            }
            13 => {
                for i in 0..cells {
                    let ptr = self.u16(head + 8 + 2 * i)? as usize;
                    out.push(self.leaf_cell(base + ptr)?);
                }
            }
            _ => return Err(GeoError::Unsupported("only table B-tree pages are read")),
        }
        Ok(())
    }

    fn leaf_cell(&self, at: usize) -> Result<Row, GeoError> {
        let (payload_len, n1) = varint(self.data, at)?;
        let (rowid, n2) = varint(self.data, at + n1)?;
        let start = at + n1 + n2;
        let total = payload_len as usize;
        let max_local = self.usable - 35;
        let min_local = (self.usable - 12) * 32 / 255 - 23;
        let local = if total <= max_local {
            total
        } else {
            let k = min_local + (total - min_local) % (self.usable - 4);
            if k <= max_local {
                k
            } else {
                min_local
            }
        };
        let mut payload = self.slice(start, local)?.to_vec();
        if total > local {
            let mut page = self.u32(start + local)?;
            let mut remaining = total - local;
            while remaining > 0 {
                if page == 0 {
                    return Err(GeoError::custom(
                        start,
                        "overflow chain ends before the payload does",
                    ));
                }
                let base = self.page_base(page)?;
                let next = self.u32(base)?;
                let take = remaining.min(self.usable - 4);
                payload.extend_from_slice(self.slice(base + 4, take)?);
                remaining -= take;
                page = next;
            }
        }
        Ok(Row { rowid: rowid as i64, values: record(&payload)? })
    }

    fn slice(&self, at: usize, len: usize) -> Result<&'a [u8], GeoError> {
        self.data.get(at..at + len).ok_or_else(|| GeoError::eof(at))
    }

    fn byte(&self, at: usize) -> Result<u8, GeoError> {
        self.data.get(at).copied().ok_or_else(|| GeoError::eof(at))
    }

    fn u16(&self, at: usize) -> Result<u16, GeoError> {
        u16_at(self.data, at).ok_or_else(|| GeoError::eof(at))
    }

    fn u32(&self, at: usize) -> Result<u32, GeoError> {
        u32_at(self.data, at).ok_or_else(|| GeoError::eof(at))
    }
}

fn u16_at(data: &[u8], at: usize) -> Option<u16> {
    let s = data.get(at..at + 2)?;
    Some(u16::from_be_bytes([s[0], s[1]]))
}

fn u32_at(data: &[u8], at: usize) -> Option<u32> {
    let s = data.get(at..at + 4)?;
    Some(u32::from_be_bytes([s[0], s[1], s[2], s[3]]))
}

/// A SQLite varint: up to nine bytes, big-endian, seven bits per byte except the ninth, which uses all eight.
fn varint(data: &[u8], at: usize) -> Result<(u64, usize), GeoError> {
    let mut v = 0u64;
    for i in 0..8 {
        let b = *data.get(at + i).ok_or_else(|| GeoError::eof(at + i))?;
        v = (v << 7) | u64::from(b & 0x7F);
        if b & 0x80 == 0 {
            return Ok((v, i + 1));
        }
    }
    let b = *data.get(at + 8).ok_or_else(|| GeoError::eof(at + 8))?;
    Ok(((v << 8) | u64::from(b), 9))
}

/// A record: a header of serial types, then the column bodies they describe.
fn record(payload: &[u8]) -> Result<Vec<Value>, GeoError> {
    let (header_len, mut at) = varint(payload, 0)?;
    let header_end = header_len as usize;
    let mut serials = Vec::new();
    while at < header_end {
        let (serial, n) = varint(payload, at)?;
        serials.push(serial);
        at += n;
    }
    let mut body = header_end;
    let mut values = Vec::with_capacity(serials.len());
    for serial in serials {
        let (value, size) = column(payload, body, serial)?;
        values.push(value);
        body += size;
    }
    Ok(values)
}

fn column(payload: &[u8], at: usize, serial: u64) -> Result<(Value, usize), GeoError> {
    let slice = |len: usize| payload.get(at..at + len).ok_or_else(|| GeoError::eof(at));
    Ok(match serial {
        0 => (Value::Null, 0),
        1..=6 => {
            let size = INT_SIZES[serial as usize];
            (Value::Int(signed(slice(size)?)), size)
        }
        7 => {
            let b = slice(8)?;
            let mut raw = [0u8; 8];
            raw.copy_from_slice(b);
            (Value::Real(f64::from_be_bytes(raw)), 8)
        }
        8 => (Value::Int(0), 0),
        9 => (Value::Int(1), 0),
        10 | 11 => return Err(GeoError::Unsupported("reserved SQLite serial type")),
        n if n % 2 == 0 => {
            let size = ((n - 12) / 2) as usize;
            (Value::Blob(slice(size)?.to_vec()), size)
        }
        n => {
            let size = ((n - 13) / 2) as usize;
            (Value::Text(String::from_utf8_lossy(slice(size)?).into_owned()), size)
        }
    })
}

/// A big-endian two's-complement integer of 1 to 8 bytes.
fn signed(b: &[u8]) -> i64 {
    let mut v = 0u64;
    for &x in b {
        v = (v << 8) | u64::from(x);
    }
    if !b.is_empty() && b.len() < 8 && b[0] & 0x80 != 0 {
        v |= !0u64 << (8 * b.len());
    }
    v as i64
}

/// The text of a value, or an empty string when it is not text.
pub(crate) fn text(v: &Value) -> String {
    match v {
        Value::Text(s) => s.clone(),
        _ => "".to_string(),
    }
}

/// The integer of a value, when it is one.
pub(crate) fn int(v: &Value) -> Option<i64> {
    match v {
        Value::Int(i) => Some(*i),
        _ => None,
    }
}
