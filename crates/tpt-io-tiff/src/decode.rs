//! TIFF LZW decoding (MSB-first codes with early change, as TIFF 6.0 specifies).

use alloc::vec::Vec;

use tpt_io_core::ParseError;

use crate::error::TiffError;

const CLEAR: u16 = 256;
const EOI: u16 = 257;

/// Decode one LZW-compressed strip.
pub fn lzw(input: &[u8]) -> Result<Vec<u8>, TiffError> {
    // Dictionary entries: (prefix code, last byte, length). Codes 0..=255 are single bytes.
    let mut prefix: Vec<u16> = Vec::with_capacity(4096);
    let mut suffix: Vec<u8> = Vec::with_capacity(4096);
    let mut length: Vec<usize> = Vec::with_capacity(4096);
    let mut out: Vec<u8> = Vec::new();
    let mut bit_pos = 0usize;
    let mut width = 9u32;
    let mut prev: Option<u16> = None;

    let reset = |prefix: &mut Vec<u16>, suffix: &mut Vec<u8>, length: &mut Vec<usize>| {
        prefix.clear();
        suffix.clear();
        length.clear();
        for b in 0..256u16 {
            prefix.push(0);
            suffix.push(b as u8);
            length.push(1);
        }
        // Codes 256 and 257 are reserved.
        prefix.push(0);
        suffix.push(0);
        length.push(0);
        prefix.push(0);
        suffix.push(0);
        length.push(0);
    };
    reset(&mut prefix, &mut suffix, &mut length);

    loop {
        let end = bit_pos + width as usize;
        if end > input.len() * 8 {
            break;
        }
        let mut code: u16 = 0;
        for k in 0..width as usize {
            let bit = (input[(bit_pos + k) / 8] >> (7 - (bit_pos + k) % 8)) & 1;
            code = (code << 1) | bit as u16;
        }
        bit_pos = end;
        if code == EOI {
            break;
        }
        if code == CLEAR {
            reset(&mut prefix, &mut suffix, &mut length);
            width = 9;
            prev = None;
            continue;
        }
        let known = prefix.len();
        let entry_index = code as usize;
        let emit = |c: u16,
                    out: &mut Vec<u8>,
                    prefix: &[u16],
                    suffix: &[u8],
                    length: &[usize]|
         -> Result<u8, TiffError> {
            let n = length[c as usize];
            let start = out.len();
            out.resize(start + n, 0);
            let mut cur = c;
            for i in (0..n).rev() {
                out[start + i] = suffix[cur as usize];
                cur = prefix[cur as usize];
            }
            Ok(out[start])
        };
        let first_byte = if entry_index < known && (length[entry_index] > 0 || entry_index < 256) {
            emit(code, &mut out, &prefix, &suffix, &length)?
        } else if entry_index == known {
            // The KwKwK case: the new entry is the previous string plus its first byte.
            let p = prev
                .ok_or_else(|| ParseError::custom(bit_pos, "LZW code before any previous code"))?;
            let p_first = emit(p, &mut out, &prefix, &suffix, &length)?;
            out.push(p_first);
            p_first
        } else {
            return Err(ParseError::custom(bit_pos, "LZW code out of range").into());
        };
        if let Some(p) = prev {
            if prefix.len() < 4096 {
                prefix.push(p);
                suffix.push(first_byte);
                length.push(length[p as usize] + 1);
            }
        }
        prev = Some(code);
        // Early change: the code width grows one code before the table reaches the boundary.
        let next = prefix.len() + 1;
        width = if next >= 2048 {
            12
        } else if next >= 1024 {
            11
        } else if next >= 512 {
            10
        } else {
            9
        };
    }
    Ok(out)
}
