//! Base64 decoding for mzML binary arrays (standard alphabet, `=` padding, whitespace ignored).

use alloc::vec::Vec;

use tpt_io_core::ParseError;

use crate::error::MzmlError;

fn value(c: u8) -> Option<u32> {
    match c {
        b'A'..=b'Z' => Some((c - b'A') as u32),
        b'a'..=b'z' => Some((c - b'a') as u32 + 26),
        b'0'..=b'9' => Some((c - b'0') as u32 + 52),
        b'+' => Some(62),
        b'/' => Some(63),
        _ => None,
    }
}

/// Decode base64 text. Whitespace is skipped; padding ends the data.
pub fn decode(text: &str) -> Result<Vec<u8>, MzmlError> {
    let mut out = Vec::with_capacity(text.len() / 4 * 3);
    let mut acc = 0u32;
    let mut bits = 0u32;
    for (i, &c) in text.as_bytes().iter().enumerate() {
        if c.is_ascii_whitespace() {
            continue;
        }
        if c == b'=' {
            break;
        }
        let v = value(c).ok_or_else(|| ParseError::invalid_byte(i, c))?;
        acc = (acc << 6) | v;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
            acc &= (1 << bits) - 1;
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_known_values_and_whitespace() {
        assert_eq!(decode("TWFu").unwrap(), b"Man");
        assert_eq!(decode("TWE=").unwrap(), b"Ma");
        assert_eq!(decode("TW\nFu ").unwrap(), b"Man");
        assert!(decode("TW!u").is_err());
    }
}
