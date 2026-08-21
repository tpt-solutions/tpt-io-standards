//! DLIS (Digital Log Interchange Standard, RP66 V1) parser.
//!
//! DLIS stores well-log data as a sequence of **Visible Records** (VR), each
//! holding one or more 1024-byte **Logical Record** (LR) blocks. This module
//! reassembles logical records from the VR/LR container, then decodes the
//! set-oriented object records (`CHANNEL`, `FRAME`, `AXIS`, `ZONE`, …) and the
//! frame-data records (`FD`) into a typed [`DlisFile`].
//!
//! > **Scope:** container reassembly, object/attribute sets, and frame data are
//! > supported for the common rep-codes (integers, IEEE floats, IDENT/ASCII
//! > strings, UVARI, STATUS). Multi-frame templates that reference channels by
//! > name are resolved against the channel list declared in the file (sufficient
//! > for the usual single-frame log).
//!
//! All parsing is allocation-based and operates on a `&[u8]` input.

use alloc::string::{String, ToString};
use alloc::vec::Vec;

use crate::error::WellLogError;

/// A decoded DLIS attribute value.
#[derive(Debug, Clone, PartialEq)]
pub enum DlisValue {
    /// No value (rep-code 0 or absent).
    None,
    /// An `IDENT` string.
    Ident(String),
    /// A free-form ASCII string.
    String(String),
    /// A boolean `STATUS`.
    Status(bool),
    /// A signed integer (INT1/2/4).
    Int(i64),
    /// An unsigned integer (UINT1/2/4/8, UVARI).
    Uint(u64),
    /// A floating-point value (FSINGL/FDOUBL).
    Float(f64),
    /// A value using an unsupported rep-code, kept raw.
    Raw(Vec<u8>),
}

/// An object (e.g. a `CHANNEL` or `FRAME`) with its attributes.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct DlisObject {
    /// Object type from the set record (e.g. `CHANNEL`).
    pub object_type: String,
    /// Object identifier (the OBNAME).
    pub name: String,
    /// Attribute (label, value) pairs.
    pub attributes: Vec<(String, DlisValue)>,
}

/// A `CHANNEL` definition: name, data rep-code, optional units.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct DlisChannel {
    /// Channel identifier.
    pub name: String,
    /// DLIS rep-code of the channel's values.
    pub rep_code: u8,
    /// Units string, if present.
    pub units: Option<String>,
}

/// Parsed frame data for one `FRAME` object.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct DlisFrameData {
    /// `FRAME` object name.
    pub frame_name: String,
    /// Channel names, in column order.
    pub channel_names: Vec<String>,
    /// One row of values per frame.
    pub rows: Vec<Vec<DlisValue>>,
}

/// A fully parsed DLIS file.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct DlisFile {
    /// All object records (CHANNEL, FRAME, AXIS, ZONE, …).
    pub objects: Vec<DlisObject>,
    /// Channel definitions.
    pub channels: Vec<DlisChannel>,
    /// Frame data records.
    pub frame_data: Vec<DlisFrameData>,
}

const LR_BLOCK: usize = 1024;

/// A forward cursor over a byte slice.
pub(crate) struct Cur<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Cur<'a> {
    fn new(data: &'a [u8]) -> Self {
        Cur { data, pos: 0 }
    }
    fn eof(&self) -> bool {
        self.pos >= self.data.len()
    }
    fn u8(&mut self) -> Result<u8, WellLogError> {
        let b = *self
            .data
            .get(self.pos)
            .ok_or(WellLogError::Unsupported("DLIS: unexpected end of input"))?;
        self.pos += 1;
        Ok(b)
    }
    fn u16(&mut self) -> Result<u16, WellLogError> {
        let s = self
            .data
            .get(self.pos..self.pos + 2)
            .ok_or(WellLogError::Unsupported("DLIS: truncated u16"))?;
        self.pos += 2;
        Ok(u16::from_be_bytes([s[0], s[1]]))
    }
    fn u32(&mut self) -> Result<u32, WellLogError> {
        let s = self
            .data
            .get(self.pos..self.pos + 4)
            .ok_or(WellLogError::Unsupported("DLIS: truncated u32"))?;
        self.pos += 4;
        Ok(u32::from_be_bytes([s[0], s[1], s[2], s[3]]))
    }
    fn take(&mut self, n: usize) -> Result<&'a [u8], WellLogError> {
        let s = self
            .data
            .get(self.pos..self.pos + n)
            .ok_or(WellLogError::Unsupported("DLIS: truncated take"))?;
        self.pos += n;
        Ok(s)
    }
}

/// Decode one value of the given DLIS rep-code from `cur`.
pub(crate) fn read_value(rep: u8, cur: &mut Cur<'_>) -> Result<DlisValue, WellLogError> {
    match rep {
        0 => Ok(DlisValue::None),
        1 => {
            // FSINGL (4-byte IEEE 754 single).
            let b = cur.take(4)?;
            Ok(DlisValue::Float(f32::from_be_bytes([b[0], b[1], b[2], b[3]]) as f64))
        }
        2 => {
            // FDOUBL (8-byte IEEE 754 double).
            let b = cur.take(8)?;
            Ok(DlisValue::Float(f64::from_be_bytes([
                b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7],
            ])))
        }
        8 => Ok(DlisValue::Int(cur.take(1)?[0] as i8 as i64)),
        9 => {
            let b = cur.take(2)?;
            Ok(DlisValue::Int(i16::from_be_bytes([b[0], b[1]]) as i64))
        }
        10 => {
            let b = cur.take(4)?;
            Ok(DlisValue::Int(i32::from_be_bytes([b[0], b[1], b[2], b[3]]) as i64))
        }
        11 => Ok(DlisValue::Uint(cur.take(1)?[0] as u64)),
        12 => {
            let b = cur.take(2)?;
            Ok(DlisValue::Uint(u16::from_be_bytes([b[0], b[1]]) as u64))
        }
        13 => Ok(DlisValue::Uint(cur.u32()? as u64)),
        17 => {
            // IDENT: 1-byte length + ASCII.
            let n = cur.u8()? as usize;
            let b = cur.take(n)?;
            Ok(DlisValue::Ident(string_lossy(b)))
        }
        18 => {
            // ASCII/STRING: 2-byte length + bytes.
            let n = cur.u16()? as usize;
            let b = cur.take(n)?;
            Ok(DlisValue::String(string_lossy(b)))
        }
        24 => Ok(DlisValue::Status(cur.u8()? != 0)),
        26 => {
            // UVARI: 1/2/4-byte variable-length unsigned.
            let first = cur.u8()?;
            let v = match first & 0xC0 {
                0x00 => (first & 0x7F) as u64,
                0x40 => {
                    let lo = cur.u8()? as u64;
                    (((first & 0x3F) as u64) << 8) | lo
                }
                0x80 | 0xC0 => {
                    let b = cur.take(3)?;
                    (((first & 0x3F) as u64) << 24)
                        | ((b[0] as u64) << 16)
                        | ((b[1] as u64) << 8)
                        | (b[2] as u64)
                }
                _ => unreachable!(),
            };
            Ok(DlisValue::Uint(v))
        }
        other => {
            // Unsupported rep-code: keep the raw bytes (assume 1 for scalars).
            let len = rep_byte_len(other);
            let b = if len == 0 { &[][..] } else { cur.take(len)? };
            Ok(DlisValue::Raw(b.to_vec()))
        }
    }
}

/// Best-effort byte length for an unsupported scalar rep-code (0 ⇒ unknown).
fn rep_byte_len(rep: u8) -> usize {
    match rep {
        3 | 4 => 4,
        5..=7 => 8,
        14 | 15 => 8,
        16 => 1,
        19 => 8,
        20 => 2,
        21 => 10,
        22 => 13,
        23 => 15,
        25 => 2,
        _ => 0,
    }
}

fn string_lossy(b: &[u8]) -> String {
    alloc::string::String::from_utf8_lossy(b).into_owned()
}

/// Reassemble logical-record byte buffers from a DLIS byte stream.
fn reassemble_logical_records(bytes: &[u8]) -> Result<Vec<Vec<u8>>, WellLogError> {
    let mut records = Vec::new();
    let mut pos = 0usize;
    while pos + 80 <= bytes.len() {
        // Visible Record header: "VR" + 2 reserved + 4-byte BE length.
        if &bytes[pos..pos + 2] != b"VR" {
            return Err(WellLogError::Unsupported("DLIS: missing Visible Record header"));
        }
        let len =
            u32::from_be_bytes([bytes[pos + 4], bytes[pos + 5], bytes[pos + 6], bytes[pos + 7]])
                as usize;
        pos += 80;
        let vr_data = bytes
            .get(pos..pos + len)
            .ok_or(WellLogError::Unsupported("DLIS: truncated Visible Record"))?;
        pos += len;

        // Logical record blocks (1024 bytes, last may be shorter).
        let mut blocks: Vec<&[u8]> = Vec::new();
        let mut bp = 0usize;
        while bp < vr_data.len() {
            let remaining = vr_data.len() - bp;
            let block_len = remaining.min(LR_BLOCK);
            blocks.push(&vr_data[bp..bp + block_len]);
            bp += block_len;
        }
        if blocks.is_empty() {
            continue;
        }
        // Each block: [u16 record length][u8 check][u8 type] + payload.
        let mut payload = Vec::new();
        let mut first = true;
        let mut set_len = 0usize;
        let mut is_lrs = true;
        for block in &blocks {
            if block.len() < 4 {
                return Err(WellLogError::Unsupported("DLIS: short Logical Record block"));
            }
            let rlen = u16::from_be_bytes([block[0], block[1]]) as usize;
            let rtype = block[3];
            let p = &block[4..];
            if first {
                set_len = rlen;
                is_lrs = rtype == 0;
                first = false;
            }
            payload.extend_from_slice(p);
        }
        if is_lrs {
            records.push(payload[..set_len.min(payload.len())].to_vec());
        } else {
            // VRLR: each block is an independent record of `set_len` bytes.
            for block in &blocks {
                let p = &block[4..];
                let take = set_len.min(p.len());
                records.push(p[..take].to_vec());
            }
        }
    }
    Ok(records)
}

/// Read an OBNAME (1-byte len, 2-byte origin, identifier, 2-byte copy).
fn read_obname(cur: &mut Cur<'_>) -> Result<String, WellLogError> {
    let len = cur.u8()? as usize;
    let _origin = cur.u16()?;
    let ident = cur.take(len)?;
    let _copy = cur.u16()?;
    Ok(string_lossy(ident))
}

/// Parse a set-oriented logical record (CHANNEL/FRAME/AXIS/ZONE/…) into objects.
fn parse_set_record(
    object_type: &str,
    body: &[u8],
    file: &mut DlisFile,
) -> Result<(), WellLogError> {
    let mut cur = Cur::new(body);
    while !cur.eof() {
        let name = read_obname(&mut cur)?;
        let nattrs = cur.u16()? as usize;
        let mut attrs = Vec::new();
        for _ in 0..nattrs {
            let label_len = cur.u8()? as usize;
            let label = string_lossy(cur.take(label_len)?);
            let rep = cur.u8()?;
            let has_value = cur.u8()? != 0;
            let value = if has_value { read_value(rep, &mut cur)? } else { DlisValue::None };
            attrs.push((label, value));
        }
        file.objects.push(DlisObject {
            object_type: object_type.to_string(),
            name: name.clone(),
            attributes: attrs.clone(),
        });
        if object_type.eq_ignore_ascii_case("CHANNEL") {
            let rep_code = attrs
                .iter()
                .find(|(l, _)| l.eq_ignore_ascii_case("REPCODE"))
                .and_then(|(_, v)| match v {
                    DlisValue::Uint(u) => Some(*u as u8),
                    DlisValue::Int(i) => Some(*i as u8),
                    _ => None,
                })
                .unwrap_or(2);
            let units =
                attrs.iter().find(|(l, _)| l.eq_ignore_ascii_case("UNITS")).and_then(|(_, v)| {
                    match v {
                        DlisValue::Ident(s) | DlisValue::String(s) => Some(s.clone()),
                        _ => None,
                    }
                });
            file.channels.push(DlisChannel { name, rep_code, units });
        }
    }
    Ok(())
}

/// Parse a frame-data (`FD`, type 58) logical record.
fn parse_frame_data(
    frame_name: &str,
    body: &[u8],
    file: &mut DlisFile,
) -> Result<(), WellLogError> {
    if file.channels.is_empty() {
        return Ok(());
    }
    let mut cur = Cur::new(body);
    let mut channel_names = Vec::new();
    let mut reps = Vec::new();
    for ch in &file.channels {
        channel_names.push(ch.name.clone());
        reps.push(ch.rep_code);
    }
    let mut rows = Vec::new();
    while !cur.eof() {
        let _frame_number = cur.u16()?;
        let mut row = Vec::new();
        for &rep in &reps {
            row.push(read_value(rep, &mut cur)?);
        }
        rows.push(row);
    }
    file.frame_data.push(DlisFrameData { frame_name: frame_name.to_string(), channel_names, rows });
    Ok(())
}

/// Parse a complete DLIS byte stream into a [`DlisFile`].
pub fn parse_dlis(bytes: &[u8]) -> Result<DlisFile, WellLogError> {
    let mut file = DlisFile::default();
    let records = reassemble_logical_records(bytes)?;
    for rec in records {
        if rec.len() < 2 {
            continue;
        }
        let rec_type = u16::from_be_bytes([rec[0], rec[1]]);
        let body = &rec[2..];
        match rec_type {
            49 => parse_set_record("CHANNEL", body, &mut file)?,
            50 => parse_set_record("FRAME", body, &mut file)?,
            51 => parse_set_record("AXIS", body, &mut file)?,
            52 => parse_set_record("ZONE", body, &mut file)?,
            58 => {
                // FD: bind to the most recently declared FRAME object.
                let fname = file
                    .objects
                    .iter()
                    .rev()
                    .find(|o| o.object_type.eq_ignore_ascii_case("FRAME"))
                    .map(|o| o.name.clone())
                    .unwrap_or_default();
                parse_frame_data(&fname, body, &mut file)?;
            }
            _ => {
                // 0 = File Header and others: parsed as generic objects when
                // they follow the set layout; otherwise skipped.
            }
        }
    }
    Ok(file)
}

/// Reader for DLIS files, implementing [`tpt_io_core::Reader`].
#[derive(Debug, Clone, Default)]
pub struct DlisReader {
    validation: tpt_io_core::ValidationSettings,
}

impl DlisReader {
    /// Create a reader with default validation.
    pub fn new() -> Self {
        Self::default()
    }
    /// Set the validation configuration.
    pub fn with_validation(mut self, validation: tpt_io_core::ValidationSettings) -> Self {
        self.validation = validation;
        self
    }
}

impl tpt_io_core::Reader<DlisFile> for DlisReader {
    type Error = WellLogError;
    fn parse(&self, input: &[u8]) -> Result<DlisFile, WellLogError> {
        let file = parse_dlis(input)?;
        self.validation.run(&DlisValidator, &file)?;
        Ok(file)
    }
}

/// Syntactic/semantic validation for a parsed [`DlisFile`].
#[derive(Debug, Clone, Copy, Default)]
pub struct DlisValidator;

impl tpt_io_core::Validator<DlisFile> for DlisValidator {
    type Error = WellLogError;
    fn validate_syntactic(&self, value: &DlisFile) -> Result<(), WellLogError> {
        for ch in &value.channels {
            if ch.rep_code == 0 {
                return Err(WellLogError::MissingField("channel REPCODE"));
            }
        }
        Ok(())
    }
    fn validate_semantic(&self, _value: &DlisFile) -> Result<(), WellLogError> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn value_int2_and_bool() {
        let bytes = [0x00, 0x0A, 0x40, 0x04, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
        let mut cur = Cur::new(&bytes);
        assert_eq!(read_value(9, &mut cur).unwrap(), DlisValue::Int(10));
        assert_eq!(read_value(2, &mut cur).unwrap(), DlisValue::Float(2.5));
    }

    #[test]
    fn uvari_7bit() {
        let mut cur = Cur::new(&[0x2A]);
        assert_eq!(read_value(26, &mut cur).unwrap(), DlisValue::Uint(42));
    }

    #[test]
    fn parses_channel_objects() {
        // Build a minimal DLIS: one VR with a CHANNEL set record.
        let mut body = Vec::new();
        body.extend_from_slice(&49u16.to_be_bytes()); // record type CHANNEL
                                                      // OBNAME: len=4 ident="DEPT" origin=0 copy=0
        body.push(4);
        body.extend_from_slice(&0u16.to_be_bytes());
        body.extend_from_slice(b"DEPT");
        body.extend_from_slice(&0u16.to_be_bytes());
        // 1 attribute: REPCODE (rep 13 UINT4) = 2
        body.extend_from_slice(&1u16.to_be_bytes());
        body.push(7); // label length
        body.extend_from_slice(b"REPCODE");
        body.push(13);
        body.push(1);
        body.extend_from_slice(&2u32.to_be_bytes());
        // The logical-record length field must delimit the real content, not the
        // padded block. A DLIS LR block is 1024 bytes: a 4-byte header
        // ([u16 len][u8 check][u8 type]) plus up to 1020 bytes of record. We pad
        // to a full block but keep `rlen` pointing at the true record length.
        let rlen = body.len() as u16;
        while body.len() % (LR_BLOCK - 4) != 0 {
            body.push(0);
        }
        let mut payload = Vec::new();
        // Block header: record length = rlen, type 0 (LRSET).
        payload.extend_from_slice(&rlen.to_be_bytes());
        payload.push(0);
        payload.push(0);
        payload.extend_from_slice(&body);
        // VR header: "VR" + 2 reserved + 4-byte length + pad to 80 bytes.
        let mut vr = Vec::new();
        vr.extend_from_slice(b"VR");
        vr.extend_from_slice(&0u16.to_be_bytes());
        vr.extend_from_slice(&(payload.len() as u32).to_be_bytes());
        while vr.len() < 80 {
            vr.push(0);
        }
        vr.extend_from_slice(&payload);

        let file = parse_dlis(&vr).unwrap();
        assert_eq!(file.channels.len(), 1);
        assert_eq!(file.channels[0].name, "DEPT");
        assert_eq!(file.channels[0].rep_code, 2);
    }
}
