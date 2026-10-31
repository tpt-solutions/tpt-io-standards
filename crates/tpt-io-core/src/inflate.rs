//! DEFLATE decoder (RFC 1951): stored, fixed-Huffman and dynamic-Huffman blocks.
//!
//! This is the decoder used by the ZIP reader. It is written with plain integer arithmetic so it works in
//! `no_std` builds with `alloc`. Malformed input returns [`ParseError`]; it never panics.

use alloc::vec::Vec;

use crate::error::{ParseError, Result};

const LEN_BASE: [u16; 29] = [
    3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59, 67, 83, 99, 115, 131, 163, 195, 227, 258,
];
const LEN_EXTRA: [u8; 29] = [0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 0];
const DIST_BASE: [u16; 30] = [
    1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513, 769, 1025, 1537, 2049, 3073, 4097, 6145,
    8193, 12289, 16385, 24577,
];
const DIST_EXTRA: [u8; 30] = [0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 12, 13, 13];
const CODE_ORDER: [usize; 19] = [16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15];

struct Bits<'a> {
    data: &'a [u8],
    pos: usize,
    bit: u32,
}

impl Bits<'_> {
    fn bit(&mut self) -> Result<u32> {
        let byte = *self.data.get(self.pos).ok_or_else(|| ParseError::eof(self.pos))?;
        let v = ((byte >> self.bit) & 1) as u32;
        self.bit += 1;
        if self.bit == 8 {
            self.bit = 0;
            self.pos += 1;
        }
        Ok(v)
    }

    fn bits(&mut self, n: u32) -> Result<u32> {
        let mut v = 0u32;
        for i in 0..n {
            v |= self.bit()? << i;
        }
        Ok(v)
    }

    fn align(&mut self) {
        if self.bit != 0 {
            self.bit = 0;
            self.pos += 1;
        }
    }
}

/// A canonical Huffman code: the number of codes of each length and the symbols in code order.
struct Huffman {
    count: [u16; 16],
    symbol: Vec<u16>,
}

impl Huffman {
    fn new(lengths: &[u8]) -> Result<Huffman> {
        let mut count = [0u16; 16];
        for &l in lengths {
            count[l as usize] += 1;
        }
        count[0] = 0;
        let mut offs = [0u16; 16];
        for len in 1..15 {
            offs[len + 1] = offs[len] + count[len];
        }
        let mut symbol = alloc::vec![0u16; lengths.len()];
        for (sym, &l) in lengths.iter().enumerate() {
            if l != 0 {
                symbol[offs[l as usize] as usize] = sym as u16;
                offs[l as usize] += 1;
            }
        }
        Ok(Huffman { count, symbol })
    }

    fn decode(&self, bits: &mut Bits<'_>) -> Result<u16> {
        let mut code: i32 = 0;
        let mut first: i32 = 0;
        let mut index: i32 = 0;
        for len in 1..16 {
            code |= bits.bit()? as i32;
            let count = self.count[len] as i32;
            if code - count < first {
                let i = index + (code - first);
                return self
                    .symbol
                    .get(i as usize)
                    .copied()
                    .ok_or_else(|| ParseError::custom(bits.pos, "Huffman code out of range"));
            }
            index += count;
            first += count;
            first <<= 1;
            code <<= 1;
        }
        Err(ParseError::custom(bits.pos, "invalid Huffman code"))
    }
}

fn fixed_tables() -> Result<(Huffman, Huffman)> {
    let mut lengths = [0u8; 288];
    for (i, l) in lengths.iter_mut().enumerate() {
        *l = match i {
            0..=143 => 8,
            144..=255 => 9,
            256..=279 => 7,
            _ => 8,
        };
    }
    let lit = Huffman::new(&lengths)?;
    let dist = Huffman::new(&[5u8; 30])?;
    Ok((lit, dist))
}

fn dynamic_tables(bits: &mut Bits<'_>) -> Result<(Huffman, Huffman)> {
    let hlit = bits.bits(5)? as usize + 257;
    let hdist = bits.bits(5)? as usize + 1;
    let hclen = bits.bits(4)? as usize + 4;
    let mut code_lengths = [0u8; 19];
    for &idx in CODE_ORDER.iter().take(hclen) {
        code_lengths[idx] = bits.bits(3)? as u8;
    }
    let code_huff = Huffman::new(&code_lengths)?;
    let mut lengths: Vec<u8> = Vec::with_capacity(hlit + hdist);
    while lengths.len() < hlit + hdist {
        let sym = code_huff.decode(bits)?;
        match sym {
            0..=15 => lengths.push(sym as u8),
            16 => {
                let prev = *lengths
                    .last()
                    .ok_or_else(|| ParseError::custom(bits.pos, "repeat with no previous length"))?;
                for _ in 0..3 + bits.bits(2)? {
                    lengths.push(prev);
                }
            }
            17 => lengths.resize(lengths.len() + 3 + bits.bits(3)? as usize, 0),
            _ => lengths.resize(lengths.len() + 11 + bits.bits(7)? as usize, 0),
        }
    }
    if lengths.len() != hlit + hdist {
        return Err(ParseError::custom(bits.pos, "code lengths overrun the table"));
    }
    let lit = Huffman::new(&lengths[..hlit])?;
    let dist = Huffman::new(&lengths[hlit..])?;
    Ok((lit, dist))
}

fn codes(bits: &mut Bits<'_>, out: &mut Vec<u8>, lit: &Huffman, dist: &Huffman) -> Result<()> {
    loop {
        let sym = lit.decode(bits)?;
        if sym < 256 {
            out.push(sym as u8);
        } else if sym == 256 {
            return Ok(());
        } else {
            let li = (sym - 257) as usize;
            if li >= LEN_BASE.len() {
                return Err(ParseError::custom(bits.pos, "invalid length symbol"));
            }
            let len = LEN_BASE[li] as usize + bits.bits(LEN_EXTRA[li] as u32)? as usize;
            let di = dist.decode(bits)? as usize;
            if di >= DIST_BASE.len() {
                return Err(ParseError::custom(bits.pos, "invalid distance symbol"));
            }
            let d = DIST_BASE[di] as usize + bits.bits(DIST_EXTRA[di] as u32)? as usize;
            if d > out.len() {
                return Err(ParseError::custom(bits.pos, "distance reaches before the output"));
            }
            let start = out.len() - d;
            for k in 0..len {
                let b = out[start + k];
                out.push(b);
            }
        }
    }
}

/// Decompress a raw DEFLATE stream (no zlib or gzip header).
pub fn inflate(data: &[u8]) -> Result<Vec<u8>> {
    let mut bits = Bits { data, pos: 0, bit: 0 };
    let mut out = Vec::new();
    loop {
        let last = bits.bit()?;
        match bits.bits(2)? {
            0 => {
                bits.align();
                let len = u16::from_le_bytes([
                    *data.get(bits.pos).ok_or_else(|| ParseError::eof(bits.pos))?,
                    *data.get(bits.pos + 1).ok_or_else(|| ParseError::eof(bits.pos))?,
                ]) as usize;
                let nlen = u16::from_le_bytes([
                    *data.get(bits.pos + 2).ok_or_else(|| ParseError::eof(bits.pos))?,
                    *data.get(bits.pos + 3).ok_or_else(|| ParseError::eof(bits.pos))?,
                ]) as usize;
                if len != !nlen & 0xFFFF {
                    return Err(ParseError::custom(bits.pos, "stored block length check failed"));
                }
                bits.pos += 4;
                let chunk = data
                    .get(bits.pos..bits.pos + len)
                    .ok_or_else(|| ParseError::eof(bits.pos))?;
                out.extend_from_slice(chunk);
                bits.pos += len;
            }
            1 => {
                let (lit, dist) = fixed_tables()?;
                codes(&mut bits, &mut out, &lit, &dist)?;
            }
            2 => {
                let (lit, dist) = dynamic_tables(&mut bits)?;
                codes(&mut bits, &mut out, &lit, &dist)?;
            }
            _ => return Err(ParseError::custom(bits.pos, "reserved block type")),
        }
        if last == 1 {
            return Ok(out);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stored_block_round_trips() {
        // A final stored block holding "abc".
        let data = [0x01, 0x03, 0x00, 0xFC, 0xFF, b'a', b'b', b'c'];
        assert_eq!(inflate(&data).unwrap(), b"abc");
    }

    #[test]
    fn fixed_block_with_back_reference() {
        // Known fixed-Huffman encoding of "aaaaaaaaaa" (a literal then a length-9 distance-1 copy).
        let data = [0x4B, 0x4C, 0x84, 0x01, 0x00];
        assert_eq!(inflate(&data).unwrap(), b"aaaaaaaaaa");
    }

    #[test]
    fn truncated_and_invalid_streams_are_errors() {
        assert!(inflate(&[]).is_err());
        assert!(inflate(&[0x07]).is_err());
        assert!(inflate(&[0x01, 0x03, 0x00, 0x00, 0x00, b'a']).is_err());
    }
}
