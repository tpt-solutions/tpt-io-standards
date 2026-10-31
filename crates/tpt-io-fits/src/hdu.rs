//! HDU structure: header blocks, data arrays, pixel decoding, and writing.

use alloc::string::ToString;
use alloc::vec;
use alloc::vec::Vec;

use tpt_io_core::ParseError;

use crate::card::{Card, Value};
use crate::error::FitsError;

const BLOCK: usize = 2880;

/// A header/data unit: its cards and its raw data bytes (padding removed).
#[derive(Debug, Clone, PartialEq)]
pub struct Hdu {
    /// Header cards in order, `END` excluded.
    pub cards: Vec<Card>,
    /// The data array as stored (big-endian), without block padding.
    pub data: Vec<u8>,
}

/// A FITS file: its HDUs in order.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Fits {
    /// Primary HDU first, then extensions.
    pub hdus: Vec<Hdu>,
}

impl Hdu {
    /// An image HDU with the given `BITPIX` and axis lengths (`NAXIS1` first). The data is empty
    /// until the caller fills it.
    pub fn image(bitpix: i64, axes: &[i64]) -> Hdu {
        let mut cards = vec![
            Card::new("SIMPLE", Value::Logical(true)),
            Card::new("BITPIX", Value::Integer(bitpix)),
            Card::new("NAXIS", Value::Integer(axes.len() as i64)),
        ];
        for (i, n) in axes.iter().enumerate() {
            cards.push(Card::new(&alloc::format!("NAXIS{}", i + 1), Value::Integer(*n)));
        }
        Hdu { cards, data: Vec::new() }
    }

    /// The first card with the given keyword.
    pub fn card(&self, keyword: &str) -> Option<&Card> {
        self.cards.iter().find(|c| c.keyword == keyword)
    }

    /// The value of an integer keyword.
    pub fn integer(&self, keyword: &str) -> Option<i64> {
        self.card(keyword).and_then(|c| c.as_integer())
    }

    /// The value of a real or integer keyword as `f64`.
    pub fn number(&self, keyword: &str) -> Option<f64> {
        match self.card(keyword)?.value.as_ref()? {
            Value::Real(r) => Some(*r),
            Value::Integer(i) => Some(*i as f64),
            _ => None,
        }
    }

    /// Axis lengths, `NAXIS1` first.
    pub fn axes(&self) -> Vec<usize> {
        let n = self.integer("NAXIS").unwrap_or(0).max(0) as usize;
        (1..=n)
            .map(|i| self.integer(&alloc::format!("NAXIS{i}")).unwrap_or(0).max(0) as usize)
            .collect()
    }

    /// Decode the image into `f64` pixels with `BZERO` and `BSCALE` applied.
    pub fn pixels(&self) -> Result<Vec<f64>, FitsError> {
        let bitpix =
            self.integer("BITPIX").ok_or_else(|| FitsError::Invalid("no BITPIX".to_string()))?;
        let width = match bitpix {
            8 => 1,
            16 => 2,
            32 | -32 => 4,
            64 | -64 => 8,
            other => return Err(FitsError::UnsupportedBitpix(other)),
        };
        let count: usize = self.axes().iter().product();
        if self.data.len() < count * width {
            return Err(FitsError::Invalid("data array is shorter than its axes".to_string()));
        }
        let bzero = self.number("BZERO").unwrap_or(0.0);
        let bscale = self.number("BSCALE").unwrap_or(1.0);
        let raw = &self.data[..count * width];
        let mut out = Vec::with_capacity(count);
        for chunk in raw.chunks_exact(width) {
            let v = match bitpix {
                8 => chunk[0] as f64,
                16 => i16::from_be_bytes([chunk[0], chunk[1]]) as f64,
                32 => i32::from_be_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]) as f64,
                64 => i64::from_be_bytes(to8(chunk)) as f64,
                -32 => f32::from_be_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]) as f64,
                _ => f64::from_be_bytes(to8(chunk)),
            };
            out.push(bzero + bscale * v);
        }
        Ok(out)
    }
}

fn to8(c: &[u8]) -> [u8; 8] {
    let mut b = [0u8; 8];
    b.copy_from_slice(c);
    b
}

/// Bytes of data an HDU occupies, from its header (before padding).
fn data_size(cards: &[Card]) -> Result<usize, FitsError> {
    let get = |k: &str| cards.iter().find(|c| c.keyword == k).and_then(|c| c.as_integer());
    let naxis = get("NAXIS").unwrap_or(0);
    if naxis < 0 {
        return Err(FitsError::Invalid("NAXIS is negative".to_string()));
    }
    if naxis == 0 {
        return Ok(0);
    }
    let bitpix = get("BITPIX").ok_or_else(|| FitsError::Invalid("no BITPIX".to_string()))?;
    let width = (bitpix.unsigned_abs() / 8) as usize;
    let mut count: usize = 1;
    for i in 1..=naxis {
        let n = get(&alloc::format!("NAXIS{i}")).unwrap_or(0);
        if n < 0 {
            return Err(FitsError::Invalid("negative axis length".to_string()));
        }
        count = count.saturating_mul(n as usize);
    }
    let pcount = get("PCOUNT").unwrap_or(0).max(0) as usize;
    let gcount = get("GCOUNT").unwrap_or(1).max(0) as usize;
    Ok(width.saturating_mul(gcount).saturating_mul(count.saturating_add(pcount)))
}

fn padded(n: usize) -> usize {
    n.div_ceil(BLOCK) * BLOCK
}

/// Parse a FITS file.
pub fn parse(input: &[u8]) -> Result<Fits, FitsError> {
    let mut pos = 0usize;
    let mut hdus = Vec::new();
    while pos < input.len() {
        let mut cards = Vec::new();
        loop {
            let block = input.get(pos..pos + BLOCK).ok_or_else(|| ParseError::eof(pos))?;
            pos += BLOCK;
            let mut ended = false;
            for image in block.chunks_exact(80) {
                let card = Card::parse(image)?;
                if card.keyword == "END" {
                    ended = true;
                    break;
                }
                cards.push(card);
            }
            if ended {
                break;
            }
        }
        let size = data_size(&cards)?;
        let total = padded(size);
        let data_end = pos.checked_add(size).ok_or_else(|| ParseError::eof(pos))?;
        let data = input.get(pos..data_end).ok_or_else(|| ParseError::eof(pos))?.to_vec();
        pos = pos.checked_add(total).ok_or_else(|| ParseError::eof(pos))?;
        if pos > input.len() {
            // Tolerate a missing final pad block when the data itself is complete.
            pos = input.len();
        }
        hdus.push(Hdu { cards, data });
    }
    if hdus.is_empty() {
        return Err(ParseError::eof(0).into());
    }
    Ok(Fits { hdus })
}

/// Serialise a FITS file. Headers are padded to whole blocks with blanks, data with zeros.
pub fn write(file: &Fits) -> Vec<u8> {
    let mut out = Vec::new();
    for hdu in &file.hdus {
        let mut header = Vec::new();
        for c in &hdu.cards {
            header.extend_from_slice(&c.to_image());
        }
        let mut end = [b' '; 80];
        end[..3].copy_from_slice(b"END");
        header.extend_from_slice(&end);
        let pad = padded(header.len()) - header.len();
        header.extend(core::iter::repeat(b' ').take(pad));
        out.extend_from_slice(&header);

        out.extend_from_slice(&hdu.data);
        let pad = padded(hdu.data.len()) - hdu.data.len();
        out.extend(core::iter::repeat(0u8).take(pad));
    }
    out
}
