//! LAS binary data parsing and input encoding detection.
//!
//! LAS files are normally ASCII, but two real-world variations occur:
//!
//! 1. **Encoding** — some exporters write the whole file as UTF-16 (LE or BE)
//!    instead of UTF-8. [`decode_input`] auto-detects the encoding from a BOM
//!    (or, failing that, a UTF-16 byte pattern) and returns a UTF-8 `String`.
//! 2. **Binary data** — a `~A` section may be stored as a packed buffer of
//!    big-endian IEEE-754 values rather than space-separated ASCII. This module
//!    interprets such a buffer given the number of curves (columns).

use alloc::string::String;
use alloc::vec::Vec;

use tpt_io_core::Encoding;

use crate::error::WellLogError;
use crate::model::WellLog;

/// Decode a raw LAS input into a UTF-8 string, auto-detecting UTF-8 vs
/// UTF-16 (LE/BE) via BOM or a heuristic byte pattern.
pub fn decode_input(input: &[u8]) -> Result<String, WellLogError> {
    match Encoding::detect(input) {
        // UTF-8 / ASCII: safe to view directly.
        enc if enc.is_str_safe() => core::str::from_utf8(input)
            .map(|s| s.to_string())
            .map_err(|_| WellLogError::Unsupported("input is not valid UTF-8")),
        // UTF-16 little-endian.
        Encoding::Utf16Le => decode_utf16(input, false),
        // UTF-16 big-endian.
        Encoding::Utf16Be => decode_utf16(input, true),
        // Anything else: best-effort UTF-8 view.
        _ => core::str::from_utf8(input)
            .map(|s| s.to_string())
            .map_err(|_| WellLogError::Unsupported("unsupported LAS encoding")),
    }
}

fn decode_utf16(input: &[u8], big_endian: bool) -> Result<String, WellLogError> {
    // Skip an optional BOM (2 bytes) before decoding units.
    let mut units: Vec<u16> = Vec::new();
    let mut i = 0;
    if input.len() >= 2
        && ((input[0] == 0xFF && input[1] == 0xFE) || (input[0] == 0xFE && input[1] == 0xFF))
    {
        i = 2;
    }
    while i + 1 < input.len() {
        let unit = if big_endian {
            u16::from_be_bytes([input[i], input[i + 1]])
        } else {
            u16::from_le_bytes([input[i], input[i + 1]])
        };
        units.push(unit);
        i += 2;
    }
    Ok(String::from_utf16_lossy(&units))
}

/// Parse a packed binary `~A` buffer into rows of `ncol` `f64` values, big-endian.
///
/// This is the common "binary LAS" layout: each value is an 8-byte IEEE-754
/// double in network (big-endian) byte order, with `ncol` values per row and no
/// padding between rows. Trailing bytes that do not form a full value are
/// ignored.
pub fn read_packed_f64(ncol: usize, bytes: &[u8]) -> Result<Vec<Vec<f64>>, WellLogError> {
    if ncol == 0 {
        return Err(WellLogError::MissingSection("C"));
    }
    let stride = ncol * 8;
    if stride == 0 {
        return Ok(Vec::new());
    }
    let mut rows = Vec::new();
    let mut off = 0;
    while off + stride <= bytes.len() {
        let mut row = Vec::with_capacity(ncol);
        for _ in 0..ncol {
            let chunk = [
                bytes[off],
                bytes[off + 1],
                bytes[off + 2],
                bytes[off + 3],
                bytes[off + 4],
                bytes[off + 5],
                bytes[off + 6],
                bytes[off + 7],
            ];
            row.push(f64::from_be_bytes(chunk));
            off += 8;
        }
        rows.push(row);
    }
    Ok(rows)
}

/// Read a [`WellLog`] whose `~A` data section is a packed binary buffer.
///
/// `header` carries the already-parsed version/well/curves; `data` is the raw
/// bytes of the `~A` section (everything after the `~A` header line).
pub fn parse_binary_log(header: WellLog, data: &[u8]) -> Result<WellLog, WellLogError> {
    let ncol = header.curves.len();
    let rows = read_packed_f64(ncol, data)?;
    Ok(WellLog { data: rows, ..header })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_utf16le_with_bom() {
        let mut bytes: Vec<u8> = vec![0xFF, 0xFE];
        for u in "VERS. 2.0".encode_utf16() {
            bytes.extend_from_slice(&u.to_le_bytes());
        }
        let s = decode_input(&bytes).unwrap();
        assert_eq!(s, "VERS. 2.0");
    }

    #[test]
    fn decodes_utf16be_without_bom() {
        // Use a non-ASCII character so the bytes cannot be misread as UTF-8.
        let mut bytes: Vec<u8> = Vec::new();
        for u in "WRAP. NÖ".encode_utf16() {
            bytes.extend_from_slice(&u.to_be_bytes());
        }
        let s = decode_input(&bytes).unwrap();
        assert_eq!(s, "WRAP. NÖ");
    }

    #[test]
    fn reads_packed_big_endian_doubles() {
        let mut buf = Vec::new();
        for v in [1.0_f64, 2.5, 3.5, 4.0, 5.0, 6.0] {
            buf.extend_from_slice(&v.to_be_bytes());
        }
        let rows = read_packed_f64(3, &buf).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0], vec![1.0, 2.5, 3.5]);
        assert_eq!(rows[1], vec![4.0, 5.0, 6.0]);
    }
}
