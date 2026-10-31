//! GRIB edition 2 messages: gridded meteorological and oceanographic data (WMO FM 92).
//!
//! A GRIB2 file is a sequence of messages, each holding one field on one grid. A message has an indicator
//! (section 0), identification (1), an optional local-use section (2), grid definition (3), product definition (4),
//! data representation (5), an optional bitmap (6), the packed data (7), and the end marker `7777`. [`parse`] reads
//! every message and unpacks its values. [`write`] writes messages with simple packing.
//!
//! Supported: latitude/longitude grids (grid definition template 3.0), analysis or forecast products (product
//! definition template 4.0), and simple packing (data representation template 5.0), with or without a bitmap.
//! Other templates are reported as [`GribError::Unsupported`].
//!
//! ```
//! use tpt_io_grib::{Message, ReferenceTime, LatLonGrid};
//!
//! let message = Message {
//!     discipline: 0,
//!     centre: 98,
//!     reference: ReferenceTime { year: 2026, month: 1, day: 2, hour: 12, minute: 0, second: 0 },
//!     category: 0,
//!     number: 0,
//!     forecast_time: 6,
//!     forecast_unit: 1,
//!     grid: LatLonGrid {
//!         ni: 2, nj: 2, lat_first: 50.0, lon_first: 0.0, lat_last: 49.0, lon_last: 1.0,
//!         di: 1.0, dj: 1.0, scanning_mode: 0,
//!     },
//!     values: vec![280.0, 281.5, 279.25, 282.0],
//! };
//! let bytes = tpt_io_grib::write(&[message]);
//! let back = tpt_io_grib::parse(&bytes).unwrap();
//! assert_eq!(back[0].values, vec![280.0, 281.5, 279.25, 282.0]);
//! ```
//!
//! Spec access: GRIB is defined by the WMO Manual on Codes (FM 92), which WMO publishes. No specification text is
//! copied into this crate. Packed values are decoded as `(R + X × 2^E) / 10^D`.

#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]

#[cfg(not(any(feature = "alloc", feature = "std")))]
compile_error!("tpt-io-grib requires the `alloc` or `std` feature: its messages own their values.");

#[cfg(any(feature = "alloc", feature = "std"))]
#[allow(unused_imports)]
#[macro_use]
extern crate alloc;

use alloc::string::{String, ToString};
use alloc::vec::Vec;

use tpt_io_core::ParseError;

/// Errors produced while reading or writing GRIB messages.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum GribError {
    /// A truncated or malformed section, with its byte offset.
    Core(ParseError),
    /// The message breaks a rule (no GRIB marker, sections that do not fit, a missing required section).
    Invalid(String),
    /// A GRIB feature this reader does not implement, such as another grid or packing template.
    Unsupported(&'static str),
}

impl From<ParseError> for GribError {
    fn from(e: ParseError) -> Self {
        GribError::Core(e)
    }
}

impl core::fmt::Display for GribError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            GribError::Core(e) => write!(f, "{e}"),
            GribError::Invalid(what) => write!(f, "invalid GRIB message: {what}"),
            GribError::Unsupported(what) => write!(f, "GRIB unsupported: {what}"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for GribError {}

/// A latitude/longitude grid (grid definition template 3.0). Angles are in degrees.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LatLonGrid {
    /// Points along a parallel (the longitude direction).
    pub ni: u32,
    /// Points along a meridian (the latitude direction).
    pub nj: u32,
    /// Latitude of the first grid point.
    pub lat_first: f64,
    /// Longitude of the first grid point.
    pub lon_first: f64,
    /// Latitude of the last grid point.
    pub lat_last: f64,
    /// Longitude of the last grid point.
    pub lon_last: f64,
    /// Longitude increment between points.
    pub di: f64,
    /// Latitude increment between points.
    pub dj: f64,
    /// The scanning mode flags. 0 means points run west to east, then north to south.
    pub scanning_mode: u8,
}

/// The reference time of a message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReferenceTime {
    /// The year.
    pub year: u16,
    /// The month, 1 to 12.
    pub month: u8,
    /// The day of the month.
    pub day: u8,
    /// The hour, 0 to 23.
    pub hour: u8,
    /// The minute.
    pub minute: u8,
    /// The second.
    pub second: u8,
}

/// One GRIB2 message: one field on one latitude/longitude grid.
#[derive(Debug, Clone, PartialEq)]
pub struct Message {
    /// The discipline from section 0, such as 0 for meteorological products.
    pub discipline: u8,
    /// The originating centre from section 1.
    pub centre: u16,
    /// The reference time from section 1.
    pub reference: ReferenceTime,
    /// The parameter category from section 4.
    pub category: u8,
    /// The parameter number from section 4.
    pub number: u8,
    /// The forecast time, in units of `forecast_unit`.
    pub forecast_time: u32,
    /// The time unit code: 0 minutes, 1 hours, 2 days, and so on.
    pub forecast_unit: u8,
    /// The grid from section 3.
    pub grid: LatLonGrid,
    /// The values in grid scanning order. A point the bitmap marks as missing is NaN.
    pub values: Vec<f64>,
}

/// Read every message in a GRIB2 file.
pub fn parse(input: &[u8]) -> Result<Vec<Message>, GribError> {
    let mut out = Vec::new();
    let mut at = 0;
    while at < input.len() {
        let (message, used) = message(&input[at..])?;
        out.push(message);
        at += used;
    }
    Ok(out)
}

/// Write messages as GRIB2 with simple packing at 16 bits per value. NaN values are written as missing, with a
/// bitmap. Each message's `values` must hold `ni × nj` entries.
pub fn write(messages: &[Message]) -> Vec<u8> {
    let mut out = Vec::new();
    for m in messages {
        out.extend_from_slice(&write_message(m));
    }
    out
}

/// Simple-packing parameters from section 5.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Packing {
    reference: f32,
    binary_scale: i32,
    decimal_scale: i32,
    bits: u8,
}

fn message(input: &[u8]) -> Result<(Message, usize), GribError> {
    if input.get(0..4) != Some(&b"GRIB"[..]) {
        return Err(GribError::Invalid("message does not start with GRIB".to_string()));
    }
    if input.get(7).copied() != Some(2) {
        return Err(GribError::Unsupported("only GRIB edition 2 is read"));
    }
    let total = be_uint(input, 8, 8)? as usize;
    if total < 20 || total > input.len() {
        return Err(ParseError::eof(input.len()).into());
    }
    let msg = &input[..total];
    if &msg[total - 4..] != b"7777" {
        return Err(GribError::Invalid("message has no 7777 end marker".to_string()));
    }
    let discipline = msg[6];

    let mut identification = None;
    let mut grid = None;
    let mut product = None;
    let mut packing = None;
    let mut bitmap = None;
    let mut data = None;
    let mut at = 16;
    while at < total - 4 {
        let len = be_uint(msg, at, 4)? as usize;
        if len < 5 || at + len > total - 4 {
            return Err(GribError::Invalid(
                "section length runs past the end of the message".to_string(),
            ));
        }
        let section = &msg[at..at + len];
        match section[4] {
            1 => identification = Some(section1(section)?),
            2 => {}
            3 => grid = Some(section3(section)?),
            4 => product = Some(section4(section)?),
            5 => packing = Some(section5(section)?),
            6 => bitmap = Some(section),
            7 => data = Some(section),
            n => return Err(GribError::Invalid(alloc::format!("unknown section {n}"))),
        }
        at += len;
    }
    if at != total - 4 {
        return Err(GribError::Invalid("sections do not end at the end marker".to_string()));
    }

    let (centre, reference) = identification
        .ok_or_else(|| GribError::Invalid("message has no identification section".to_string()))?;
    let grid =
        grid.ok_or_else(|| GribError::Invalid("message has no grid definition".to_string()))?;
    let (category, number, forecast_unit, forecast_time) = product
        .ok_or_else(|| GribError::Invalid("message has no product definition".to_string()))?;
    let packing = packing
        .ok_or_else(|| GribError::Invalid("message has no data representation".to_string()))?;
    let data = data.ok_or_else(|| GribError::Invalid("message has no data section".to_string()))?;

    let points = (grid.ni as usize)
        .checked_mul(grid.nj as usize)
        .filter(|&n| n > 0)
        .ok_or_else(|| GribError::Invalid("grid has no points".to_string()))?;
    let values = unpack_values(points, packing, bitmap, data)?;
    let message = Message {
        discipline,
        centre,
        reference,
        category,
        number,
        forecast_time,
        forecast_unit,
        grid,
        values,
    };
    Ok((message, total))
}

fn section1(s: &[u8]) -> Result<(u16, ReferenceTime), GribError> {
    if s.len() < 21 {
        return Err(ParseError::eof(s.len()).into());
    }
    let centre = be_uint(s, 5, 2)? as u16;
    let reference = ReferenceTime {
        year: be_uint(s, 12, 2)? as u16,
        month: s[14],
        day: s[15],
        hour: s[16],
        minute: s[17],
        second: s[18],
    };
    Ok((centre, reference))
}

fn section3(s: &[u8]) -> Result<LatLonGrid, GribError> {
    if s.len() < 72 {
        return Err(ParseError::eof(s.len()).into());
    }
    if be_uint(s, 12, 2)? != 0 {
        return Err(GribError::Unsupported(
            "only grid definition template 3.0 (latitude/longitude) is read",
        ));
    }
    if be_uint(s, 38, 4)? != 0 {
        return Err(GribError::Unsupported("a non-default basic angle is not read"));
    }
    Ok(LatLonGrid {
        ni: be_uint(s, 30, 4)? as u32,
        nj: be_uint(s, 34, 4)? as u32,
        lat_first: angle(s, 46)?,
        lon_first: angle(s, 50)?,
        lat_last: angle(s, 55)?,
        lon_last: angle(s, 59)?,
        di: angle(s, 63)?,
        dj: angle(s, 67)?,
        scanning_mode: s[71],
    })
}

/// Section 4 for product template 4.0: returns category, number, time unit and forecast time.
fn section4(s: &[u8]) -> Result<(u8, u8, u8, u32), GribError> {
    if s.len() < 22 {
        return Err(ParseError::eof(s.len()).into());
    }
    if be_uint(s, 7, 2)? != 0 {
        return Err(GribError::Unsupported("only product definition template 4.0 is read"));
    }
    Ok((s[9], s[10], s[17], be_uint(s, 18, 4)? as u32))
}

fn section5(s: &[u8]) -> Result<Packing, GribError> {
    if s.len() < 21 {
        return Err(ParseError::eof(s.len()).into());
    }
    if be_uint(s, 9, 2)? != 0 {
        return Err(GribError::Unsupported(
            "only data representation template 5.0 (simple packing) is read",
        ));
    }
    Ok(Packing {
        reference: f32::from_bits(be_uint(s, 11, 4)? as u32),
        binary_scale: signed16(be_uint(s, 15, 2)?),
        decimal_scale: signed16(be_uint(s, 17, 2)?),
        bits: s[19],
    })
}

/// Unpack the values of a message: the points marked present in the bitmap (all of them when there is none) are
/// decoded from the packed data, and the rest are NaN.
fn unpack_values(
    points: usize,
    packing: Packing,
    bitmap: Option<&[u8]>,
    data: &[u8],
) -> Result<Vec<f64>, GribError> {
    let present: Vec<bool> = match bitmap {
        None => alloc::vec![true; points],
        Some(s) => {
            if s.len() < 6 {
                return Err(ParseError::eof(s.len()).into());
            }
            match s[5] {
                255 => alloc::vec![true; points],
                0 => unpack_bits(&s[6..], 1, points)?.into_iter().map(|b| b == 1).collect(),
                _ => return Err(GribError::Unsupported("predefined bitmaps are not read")),
            }
        }
    };
    let packed = present.iter().filter(|&&p| p).count();
    let raw = unpack_bits(data.get(5..).unwrap_or(&[]), packing.bits as usize, packed)?;
    let reference = packing.reference as f64;
    let scale = 2f64.powi(packing.binary_scale);
    let divisor = 10f64.powi(packing.decimal_scale);
    let mut out = Vec::with_capacity(points);
    let mut next = raw.into_iter();
    for p in present {
        if p {
            let x = next.next().unwrap_or(0) as f64;
            out.push((reference + x * scale) / divisor);
        } else {
            out.push(f64::NAN);
        }
    }
    Ok(out)
}

/// Read `count` unsigned values of `width` bits each, most significant bit first.
fn unpack_bits(data: &[u8], width: usize, count: usize) -> Result<Vec<u64>, GribError> {
    if width > 64 {
        return Err(GribError::Unsupported("packed values wider than 64 bits"));
    }
    if width == 0 {
        return Ok(alloc::vec![0; count]);
    }
    let needed = width
        .checked_mul(count)
        .ok_or_else(|| GribError::Invalid("packed data is too large".to_string()))?;
    if needed > data.len() * 8 {
        return Err(ParseError::eof(data.len()).into());
    }
    let mut out = Vec::with_capacity(count);
    let mut bit = 0usize;
    for _ in 0..count {
        let mut v = 0u64;
        for _ in 0..width {
            let byte = data[bit / 8];
            v = (v << 1) | u64::from((byte >> (7 - bit % 8)) & 1);
            bit += 1;
        }
        out.push(v);
    }
    Ok(out)
}

fn be_uint(s: &[u8], at: usize, width: usize) -> Result<u64, GribError> {
    let bytes = s.get(at..at + width).ok_or_else(|| ParseError::eof(at))?;
    Ok(bytes.iter().fold(0u64, |acc, &b| (acc << 8) | u64::from(b)))
}

/// A signed 16-bit value in sign-and-magnitude form, as GRIB stores scale factors.
fn signed16(raw: u64) -> i32 {
    let mag = (raw & 0x7FFF) as i32;
    if raw & 0x8000 != 0 {
        -mag
    } else {
        mag
    }
}

/// An angle in micro-degrees, stored as a 32-bit sign-and-magnitude integer.
fn angle(s: &[u8], at: usize) -> Result<f64, GribError> {
    let raw = be_uint(s, at, 4)? as u32;
    let mag = (raw & 0x7FFF_FFFF) as f64;
    let micro = if raw & 0x8000_0000 != 0 { -mag } else { mag };
    Ok(micro / 1_000_000.0)
}

fn encode_angle(deg: f64) -> u32 {
    let micro = (deg * 1_000_000.0).round() as i64;
    let mag = (micro.unsigned_abs() as u32) & 0x7FFF_FFFF;
    if micro < 0 {
        mag | 0x8000_0000
    } else {
        mag
    }
}

fn encode_signed16(v: i32) -> u64 {
    if v < 0 {
        0x8000 | (-v) as u64
    } else {
        v as u64
    }
}

/// Set a big-endian value at a 1-based octet, as the spec numbers them.
fn put(buf: &mut [u8], octet: usize, width: usize, value: u64) {
    let bytes = value.to_be_bytes();
    buf[octet - 1..octet - 1 + width].copy_from_slice(&bytes[8 - width..]);
}

/// A section of `len` bytes with its length and number set. Its other fields are set with [`put`].
fn blank(number: u8, len: usize) -> Vec<u8> {
    let mut s = vec![0u8; len];
    put(&mut s, 1, 4, len as u64);
    s[4] = number;
    s
}

/// The smallest binary scale whose 16-bit range covers `range`.
fn choose_binary_scale(range: f64) -> i32 {
    if range == 0.0 {
        return 0;
    }
    let mut e: i32 = -200;
    while range * 2f64.powi(-e) > 65535.0 {
        e += 1;
    }
    e
}

fn write_message(m: &Message) -> Vec<u8> {
    let g = m.grid;
    let points = g.ni as usize * g.nj as usize;
    let values: Vec<f64> = m.values.iter().take(points).copied().collect();
    let present: Vec<f64> = values.iter().copied().filter(|v| !v.is_nan()).collect();
    let (lo, hi) = present
        .iter()
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), &v| (lo.min(v), hi.max(v)));
    let (lo, hi) = if present.is_empty() { (0.0, 0.0) } else { (lo, hi) };
    let reference = lo as f32;
    let binary_scale = choose_binary_scale(hi - reference as f64);
    let scale = 2f64.powi(binary_scale);
    let mut data = Vec::with_capacity(present.len() * 2);
    for &v in &present {
        let x = ((v - reference as f64) / scale).round().clamp(0.0, 65535.0) as u16;
        data.extend_from_slice(&x.to_be_bytes());
    }
    let missing = present.len() < values.len();

    // Section 1: identification, 21 bytes.
    let mut s1 = blank(1, 21);
    put(&mut s1, 6, 2, m.centre as u64);
    put(&mut s1, 12, 1, 1);
    put(&mut s1, 13, 2, m.reference.year as u64);
    put(&mut s1, 15, 1, m.reference.month as u64);
    put(&mut s1, 16, 1, m.reference.day as u64);
    put(&mut s1, 17, 1, m.reference.hour as u64);
    put(&mut s1, 18, 1, m.reference.minute as u64);
    put(&mut s1, 19, 1, m.reference.second as u64);

    // Section 3: grid definition, template 3.0, 72 bytes.
    let mut s3 = blank(3, 72);
    put(&mut s3, 7, 4, points as u64);
    put(&mut s3, 15, 1, 6);
    put(&mut s3, 31, 4, g.ni as u64);
    put(&mut s3, 35, 4, g.nj as u64);
    put(&mut s3, 47, 4, encode_angle(g.lat_first) as u64);
    put(&mut s3, 51, 4, encode_angle(g.lon_first) as u64);
    put(&mut s3, 55, 1, 0x30);
    put(&mut s3, 56, 4, encode_angle(g.lat_last) as u64);
    put(&mut s3, 60, 4, encode_angle(g.lon_last) as u64);
    put(&mut s3, 64, 4, encode_angle(g.di) as u64);
    put(&mut s3, 68, 4, encode_angle(g.dj) as u64);
    put(&mut s3, 72, 1, g.scanning_mode as u64);

    // Section 4: product definition, template 4.0, 34 bytes.
    let mut s4 = blank(4, 34);
    put(&mut s4, 10, 1, m.category as u64);
    put(&mut s4, 11, 1, m.number as u64);
    put(&mut s4, 18, 1, m.forecast_unit as u64);
    put(&mut s4, 19, 4, m.forecast_time as u64);
    put(&mut s4, 23, 1, 1);
    put(&mut s4, 29, 1, 255);

    // Section 5: data representation, template 5.0, 21 bytes.
    let mut s5 = blank(5, 21);
    put(&mut s5, 6, 4, present.len() as u64);
    put(&mut s5, 12, 4, reference.to_bits() as u64);
    put(&mut s5, 16, 2, encode_signed16(binary_scale));
    put(&mut s5, 18, 2, encode_signed16(0));
    put(&mut s5, 20, 1, 16);

    // Section 6: bitmap, present only when some values are missing.
    let s6 = if missing {
        let mut s = blank(6, 6 + points.div_ceil(8));
        put(&mut s, 6, 1, 0);
        for (i, v) in values.iter().enumerate() {
            if !v.is_nan() {
                s[6 + i / 8] |= 0x80 >> (i % 8);
            }
        }
        s
    } else {
        let mut s = blank(6, 6);
        put(&mut s, 6, 1, 255);
        s
    };

    let mut s7 = blank(7, 5 + data.len());
    s7[5..].copy_from_slice(&data);

    let mut out = Vec::new();
    out.extend_from_slice(b"GRIB");
    out.extend_from_slice(&[0, 0, m.discipline, 2]);
    out.extend_from_slice(&[0u8; 8]);
    for s in [&s1, &s3, &s4, &s5, &s6, &s7] {
        out.extend_from_slice(s);
    }
    out.extend_from_slice(b"7777");
    let total = out.len() as u64;
    out[8..16].copy_from_slice(&total.to_be_bytes());
    out
}
