//! Data block framing, FSPEC decoding and the CAT048 field subset.

use alloc::vec::Vec;

use tpt_io_core::ParseError;

use crate::error::AsterixError;

/// One data block: its category and its records.
#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    /// The category number, e.g. 48.
    pub category: u8,
    /// Records in order. Only category 048 is decoded.
    pub records: Vec<Record>,
}

/// A decoded record.
#[derive(Debug, Clone, PartialEq)]
pub enum Record {
    /// A decoded CAT048 monoradar target report.
    Cat048(Cat048Record),
}

/// The CAT048 fields this crate decodes. Each is `None` when the record does not carry it.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Cat048Record {
    /// I048/010: system area code and system identification code.
    pub data_source: Option<(u8, u8)>,
    /// I048/140: time of day in 1/128 second units.
    pub time_of_day: Option<u32>,
    /// I048/040: measured position as (range in NM, azimuth in degrees).
    pub polar_position: Option<(f64, f64)>,
    /// I048/070: Mode-3/A code, the 12 significant bits.
    pub mode3a: Option<u16>,
    /// I048/090: flight level in units of 1/4 FL (2 bytes, 14 significant bits).
    pub flight_level_quarters: Option<u16>,
    /// I048/220: 24-bit aircraft address.
    pub aircraft_address: Option<u32>,
    /// I048/161: track number (12 significant bits).
    pub track_number: Option<u16>,
}

impl Record {
    /// The time of day in seconds, if the record carries I048/140.
    pub fn time_of_day_s(&self) -> Option<f64> {
        match self {
            Record::Cat048(r) => r.time_of_day.map(|t| t as f64 / 128.0),
        }
    }
}

/// The item lengths of CAT048 that this crate can decode, by FRN (1-based).
/// `None` for an FRN means the item is not supported.
fn cat048_length(frn: usize) -> Option<usize> {
    match frn {
        1 => Some(2),  // I048/010 data source
        2 => Some(3),  // I048/140 time of day
        4 => Some(4),  // I048/040 measured position (polar)
        5 => Some(2),  // I048/070 Mode-3/A
        6 => Some(2),  // I048/090 flight level
        8 => Some(3),  // I048/220 aircraft address
        11 => Some(2), // I048/161 track number
        _ => None,
    }
}

/// Parse a sequence of ASTERIX data blocks.
pub fn parse(input: &[u8]) -> Result<Vec<Block>, AsterixError> {
    let mut blocks = Vec::new();
    let mut pos = 0usize;
    while pos < input.len() {
        if pos + 3 > input.len() {
            return Err(ParseError::eof(pos).into());
        }
        let category = input[pos];
        let len = u16::from_be_bytes([input[pos + 1], input[pos + 2]]) as usize;
        if len < 3 || pos + len > input.len() {
            return Err(ParseError::custom(pos, "data block length is out of range").into());
        }
        let body = &input[pos + 3..pos + len];
        blocks.push(Block { category, records: records(category, body, pos + 3)? });
        pos += len;
    }
    Ok(blocks)
}

/// Read the FSPEC of a record: octets whose top seven bits flag items and whose low bit is FX.
fn fspec(body: &[u8], at: usize) -> Result<(Vec<usize>, usize), AsterixError> {
    let mut frns = Vec::new();
    let mut i = 0usize;
    loop {
        let octet = *body.get(i).ok_or_else(|| ParseError::eof(at + i))?;
        for bit in 0..7 {
            if octet & (0x80 >> bit) != 0 {
                frns.push(i * 7 + bit + 1);
            }
        }
        i += 1;
        if octet & 0x01 == 0 {
            return Ok((frns, i));
        }
    }
}

fn records(category: u8, body: &[u8], at: usize) -> Result<Vec<Record>, AsterixError> {
    let mut out = Vec::new();
    let mut pos = 0usize;
    while pos < body.len() {
        let (frns, fspec_len) = fspec(&body[pos..], at + pos)?;
        pos += fspec_len;
        if category != 48 {
            // Without item lengths for this category, the record boundary is unknown.
            return Err(AsterixError::UnsupportedItem { category, frn: frns.first().copied().unwrap_or(1) });
        }
        let mut rec = Cat048Record::default();
        for frn in frns {
            let len = cat048_length(frn).ok_or(AsterixError::UnsupportedItem { category, frn })?;
            let field = body
                .get(pos..pos + len)
                .ok_or_else(|| ParseError::eof(at + pos))?;
            set(&mut rec, frn, field);
            pos += len;
        }
        out.push(Record::Cat048(rec));
    }
    Ok(out)
}

fn set(rec: &mut Cat048Record, frn: usize, f: &[u8]) {
    match frn {
        1 => rec.data_source = Some((f[0], f[1])),
        2 => rec.time_of_day = Some(u32::from_be_bytes([0, f[0], f[1], f[2]])),
        4 => {
            let rho = u16::from_be_bytes([f[0], f[1]]) as f64 / 256.0;
            let theta = u16::from_be_bytes([f[2], f[3]]) as f64 * 360.0 / 65536.0;
            rec.polar_position = Some((rho, theta));
        }
        5 => rec.mode3a = Some(u16::from_be_bytes([f[0], f[1]]) & 0x0FFF),
        6 => rec.flight_level_quarters = Some(u16::from_be_bytes([f[0], f[1]]) & 0x3FFF),
        8 => rec.aircraft_address = Some(u32::from_be_bytes([0, f[0], f[1], f[2]])),
        11 => rec.track_number = Some(u16::from_be_bytes([f[0], f[1]]) & 0x0FFF),
        _ => {}
    }
}
