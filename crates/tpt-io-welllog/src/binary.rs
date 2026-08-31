//! LAS 3.0 binary data-section parsing and automatic encoding detection.
//!
//! LAS 2.0 is purely ASCII, but LAS 3.0 extends the `~A` section so the data
//! may be stored as raw binary after an `BINARY` keyword line. Per the LAS 3.0
//! spec the curve *data type* lives in the `~C` section's API-code column
//! (e.g. `F4`, `F8`, `I2`, `U4`, `S32`), and every record is a fixed-width pack
//! of one value per curve.
//!
//! Automatic encoding detection reuses [`tpt_io_core::Encoding`] and transcodes
//! UTF-16 inputs to UTF-8 so the (text) header is always parseable; the binary
//! tail is then read directly from the original bytes.

use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

use tpt_io_core::Encoding;

use crate::error::WellLogError;
use crate::model::WellLog;
use crate::parser::{parse_ascii_data, parse_header};

/// Maximum binary record width we are willing to accept (defends against a
/// malformed header producing an absurd stride and reading out of bounds).
const MAX_ROW_WIDTH: usize = 1 << 16;

/// Decode a LAS byte buffer into a `&str`-compatible [`String`], transcoding
/// UTF-16 (LE/BE) when detected. Returns the decoded text and the detected
/// encoding.
pub(crate) fn decode_to_string(bytes: &[u8]) -> Result<(String, Encoding), WellLogError> {
    let enc = Encoding::detect(bytes);
    match enc {
        Encoding::Ascii | Encoding::Utf8 => {
            let s = enc
                .as_str(bytes)
                .ok_or(WellLogError::Unsupported("input was not valid UTF-8"))?;
            Ok((s.to_string(), enc))
        }
        Encoding::Utf16Le => Ok((utf16_to_string(bytes, false)?, enc)),
        Encoding::Utf16Be => Ok((utf16_to_string(bytes, true)?, enc)),
    }
}

/// Transcode a UTF-16 byte slice (optionally BOM-prefixed) into a Rust [`String`].
fn utf16_to_string(bytes: &[u8], be: bool) -> Result<String, WellLogError> {
    if bytes.len() < 2 || bytes.len() % 2 != 0 {
        return Err(WellLogError::Unsupported("truncated UTF-16 input"));
    }
    let mut start = 0;
    // Strip a leading UTF-16 BOM if present.
    if bytes.starts_with(&[0xFF, 0xFE]) || bytes.starts_with(&[0xFE, 0xFF]) {
        start = 2;
    }
    let mut units: Vec<u16> = Vec::with_capacity((bytes.len() - start) / 2);
    let mut i = start;
    while i + 1 < bytes.len() {
        let (hi, lo) = (bytes[i] as u16, bytes[i + 1] as u16);
        let unit = if be { (hi << 8) | lo } else { hi | (lo << 8) };
        units.push(unit);
        i += 2;
    }
    let s: String = core::char::decode_utf16(units)
        .map(|r| r.unwrap_or(char::REPLACEMENT_CHARACTER))
        .collect();
    Ok(s)
}

/// A LAS 3.0 binary curve data type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BinaryType {
    /// 4-byte little-endian IEEE float.
    F4,
    /// 8-byte little-endian IEEE double.
    F8,
    /// 1-byte signed integer.
    I1,
    /// 2-byte little-endian signed integer.
    I2,
    /// 4-byte little-endian signed integer.
    I4,
    /// 1-byte unsigned integer.
    U1,
    /// 2-byte little-endian unsigned integer.
    U2,
    /// 4-byte little-endian unsigned integer.
    U4,
    /// Fixed-width ASCII string of `n` bytes (stored as 0.0 / uninterpreted).
    Str(usize),
}

impl BinaryType {
    /// Parse a `~C` API-code value into a [`BinaryType`].
    fn parse(code: &str) -> Option<BinaryType> {
        match code.trim().to_ascii_uppercase().as_str() {
            "F4" | "F" => Some(BinaryType::F4),
            "F8" | "D" => Some(BinaryType::F8),
            "I1" => Some(BinaryType::I1),
            "I2" => Some(BinaryType::I2),
            "I4" => Some(BinaryType::I4),
            "U1" | "B" => Some(BinaryType::U1),
            "U2" => Some(BinaryType::U2),
            "U4" => Some(BinaryType::U4),
            other => other
                .strip_prefix('S')
                .and_then(|s| s.parse::<usize>().ok())
                .map(BinaryType::Str),
        }
    }

    /// Byte width of one value of this type.
    fn width(self) -> usize {
        match self {
            BinaryType::F4 | BinaryType::I4 | BinaryType::U4 => 4,
            BinaryType::F8 => 8,
            BinaryType::I2 | BinaryType::U2 => 2,
            BinaryType::I1 | BinaryType::U1 => 1,
            BinaryType::Str(n) => n,
        }
    }
}

/// Parse a complete LAS 2.0/3.0 document from raw bytes, transparently handling
/// both ASCII and binary `~A` sections and any detected text encoding.
pub fn parse(input: &[u8]) -> Result<WellLog, WellLogError> {
    // Fast path: the entire buffer is valid UTF-8/ASCII. A true binary tail
    // would break UTF-8, so this is a text LAS (ASCII or UTF-8).
    if let Ok(text) = core::str::from_utf8(input) {
        let (header, remainder) = parse_header(text)?;
        return parse_ascii_data(header, remainder);
    }

    // Not wholly valid UTF-8. This is either (a) an ASCII/UTF-8 header followed
    // by a raw binary `~A` tail, or (b) a whole-file UTF-16 document.
    if let Some(bin_line_end) = find_binary_marker(input) {
        // (a) ASCII/UTF-8 header with a binary tail. The header bytes are valid
        // UTF-8 (text), so slice and decode just that prefix.
        let header_text = core::str::from_utf8(&input[..bin_line_end])
            .map_err(|_| WellLogError::Unsupported("LAS header is not valid UTF-8"))?;
        let (header, _remainder) = parse_header(header_text)?;
        return parse_binary_tail(header, &input[bin_line_end..]);
    }

    // (b) Whole-file UTF-16 (or other encoding): transcode and parse as text.
    let (text, enc) = decode_to_string(input)?;
    let (header, remainder) = parse_header(&text)?;
    let keyword = remainder
        .lines()
        .map(|l| l.trim())
        .find(|l| !l.is_empty())
        .unwrap_or("");
    let is_binary = keyword.len() <= 7
        && keyword.chars().all(|c| c.is_ascii_alphabetic())
        && keyword.eq_ignore_ascii_case("BINARY");
    if is_binary {
        // UTF-16 binary tails cannot be located by text offset; only ASCII/UTF-8
        // binary tails are supported.
        if !matches!(enc, Encoding::Ascii | Encoding::Utf8) {
            return Err(WellLogError::Unsupported(
                "binary LAS data is not supported for UTF-16 source files",
            ));
        }
        let rem_start = remainder.as_ptr() as usize - text.as_ptr() as usize;
        let bin_start = match text[rem_start..].find('\n') {
            Some(rel) => rem_start + rel + 1,
            None => text.len(),
        };
        return parse_binary_tail(header, &input[bin_start..]);
    }
    parse_ascii_data(header, remainder)
}

/// Locate the byte offset immediately after the `BINARY` keyword line in a raw
/// (possibly binary-tailed) LAS buffer. Returns `None` if no such marker exists.
fn find_binary_marker(input: &[u8]) -> Option<usize> {
    let pat: &[u8] = b"BINARY";
    let mut i = 0;
    while i + pat.len() <= input.len() {
        if &input[i..i + pat.len()] == pat {
            let line_start = i == 0 || input[i - 1] == b'\n' || input[i - 1] == b'\r';
            if line_start {
                let mut j = i + pat.len();
                while j < input.len() && input[j] != b'\n' {
                    j += 1;
                }
                return Some(if j < input.len() { j + 1 } else { input.len() });
            }
        }
        i += 1;
    }
    None
}

/// Read the fixed-width binary records described by the parsed curve types.
fn parse_binary_tail(
    header: crate::parser::ParsedHeader,
    tail: &[u8],
) -> Result<WellLog, WellLogError> {
    let curve_types: Vec<Option<BinaryType>> = header
        .curves
        .iter()
        .map(|c| BinaryType::parse(&c.api_code))
        .collect();

    if curve_types.iter().any(|t| t.is_none()) {
        return Err(WellLogError::Unsupported(
            "one or more curves declare an unsupported binary data type",
        ));
    }
    let widths: Vec<usize> = curve_types.iter().map(|t| t.unwrap().width()).collect();
    let row_width: usize = widths.iter().sum();
    if row_width == 0 || row_width > MAX_ROW_WIDTH || tail.len() % row_width != 0 {
        return Err(WellLogError::InvalidNumber {
            offset: 0,
            value: format!("binary row width {row_width} vs {} bytes", tail.len()),
        });
    }

    let ncol = header.curves.len();
    let nrows = tail.len() / row_width;
    let mut data: Vec<Vec<f64>> = Vec::with_capacity(nrows);

    for r in 0..nrows {
        let base = r * row_width;
        let mut row = Vec::with_capacity(ncol);
        let mut off = 0usize;
        for col in 0..ncol {
            let w = widths[col];
            let slice = &tail[base + off..base + off + w];
            off += w;
            let v = read_value(curve_types[col].unwrap(), slice)?;
            row.push(v);
        }
        data.push(row);
    }

    Ok(WellLog {
        version: header.version,
        wrap: header.wrap,
        well: header.well,
        parameters: header.parameters,
        curves: header.curves,
        data,
    })
}

/// Decode a single little-endian scalar of `t` from exactly `slice`.
fn read_value(t: BinaryType, slice: &[u8]) -> Result<f64, WellLogError> {
    let n = slice.len();
    let too_short = n < t.width();
    let v = match t {
        BinaryType::F4 => {
            if too_short {
                return Err(short(t, n));
            }
            f32::from_le_bytes([slice[0], slice[1], slice[2], slice[3]]) as f64
        }
        BinaryType::F8 => {
            if too_short {
                return Err(short(t, n));
            }
            f64::from_le_bytes(slice.try_into().unwrap())
        }
        BinaryType::I1 => i8::from_le_bytes([slice[0]]) as f64,
        BinaryType::I2 => {
            if too_short {
                return Err(short(t, n));
            }
            i16::from_le_bytes([slice[0], slice[1]]) as f64
        }
        BinaryType::I4 => {
            if too_short {
                return Err(short(t, n));
            }
            i32::from_le_bytes([slice[0], slice[1], slice[2], slice[3]]) as f64
        }
        BinaryType::U1 => slice[0] as f64,
        BinaryType::U2 => {
            if too_short {
                return Err(short(t, n));
            }
            u16::from_le_bytes([slice[0], slice[1]]) as f64
        }
        BinaryType::U4 => {
            if too_short {
                return Err(short(t, n));
            }
            u32::from_le_bytes([slice[0], slice[1], slice[2], slice[3]]) as f64
        }
        BinaryType::Str(_) => {
            // Strings cannot be represented on the numeric matrix; attempt a
            // numeric parse, else emit the null value sentinel 0.0.
            let s = core::str::from_utf8(slice)
                .map(|s| s.trim().to_string())
                .unwrap_or_default();
            s.parse::<f64>().unwrap_or(0.0)
        }
    };
    Ok(v)
}

fn short(t: BinaryType, n: usize) -> WellLogError {
    WellLogError::InvalidNumber {
        offset: 0,
        value: format!("binary type {t:?} needs {} bytes, got {n}", t.width()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const HEADER_ASCII: &str = "\
~Version Information
VERS. 3.0 :
WRAP. NO :
~Well Information
NULL. -999.25 :
WELL. BINARY TEST :
~Curve Information
DEPT.M   F4 : DEPTH
GR.API   F4 : GAMMA RAY
~ASCII Log Data
BINARY
";

    fn build_binary_log() -> Vec<u8> {
        let mut bytes = HEADER_ASCII.as_bytes().to_vec();
        // Three rows: DEPT (f32) + GR (f32), little-endian.
        for (d, g) in [(10.0f32, 75.5f32), (20.0, 80.1), (30.0, 78.4)] {
            bytes.extend_from_slice(&d.to_le_bytes());
            bytes.extend_from_slice(&g.to_le_bytes());
        }
        bytes
    }

    #[test]
    fn parses_binary_las3() {
        let log = parse(&build_binary_log()).unwrap();
        assert_eq!(log.version, 3.0);
        assert_eq!(log.curves.len(), 2);
        assert_eq!(log.data.len(), 3);
        assert_eq!(log.data[0], vec![10.0, 75.5]);
        assert!((log.data[2][1] - 78.4).abs() < 1e-3, "got {:?}", log.data[2]);
    }

    #[test]
    fn detects_utf16_and_transcodes_header() {
        // Build a UTF-16LE LAS 2.0 ASCII file (header + ASCII data).
        let ascii = "~Version Information\nVERS. 2.0 :\nWRAP. NO :\n~Well Information\nWELL. X :\n~Curve Information\nDEPT. M :\nGR . API :\n~ASCII Log Data\n1.0 2.0\n3.0 4.0\n";
        let mut bytes: Vec<u8> = Vec::new();
        bytes.push(0xFF);
        bytes.push(0xFE); // UTF-16LE BOM
        for ch in ascii.chars() {
            bytes.push(ch as u8);
            bytes.push(0);
        }
        let log = parse(&bytes).unwrap();
        assert_eq!(log.well.well_name.as_deref(), Some("X"));
        assert_eq!(log.data, vec![vec![1.0, 2.0], vec![3.0, 4.0]]);
    }

    #[test]
    fn binary_with_bad_width_errors() {
        let mut bytes = HEADER_ASCII.as_bytes().to_vec();
        bytes.extend_from_slice(&[0u8; 5]); // not a multiple of 8
        assert!(parse(&bytes).is_err());
    }
}
