//! SEG-Y headers, traces, sample decoding and writing.

use alloc::vec::Vec;

use tpt_io_core::ParseError;

use crate::error::SegyError;

const TEXT_LEN: usize = 3200;
const BIN_LEN: usize = 400;
const TRACE_HEADER_LEN: usize = 240;

/// The fields of the 400-byte binary header that this crate uses.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct BinaryHeader {
    /// Sample interval in microseconds (bytes 3217–3218).
    pub sample_interval_us: u16,
    /// Samples per trace (bytes 3221–3222).
    pub samples_per_trace: u16,
    /// Data sample format code (bytes 3225–3226): 1 IBM float, 2 int32, 3 int16, 5 IEEE float, 8 int8.
    pub format: u16,
    /// The 400 bytes exactly as read, so unused fields round-trip.
    pub raw: Vec<u8>,
}

/// One trace: its raw 240-byte header and its samples as `f32`.
#[derive(Debug, Clone, PartialEq)]
pub struct Trace {
    /// The 240-byte trace header, as read.
    pub header: [u8; TRACE_HEADER_LEN],
    /// Samples in file order.
    pub samples: Vec<f32>,
}

/// A SEG-Y file.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SegY {
    /// The 3200-byte textual header, as read.
    pub text: Vec<u8>,
    /// The binary header.
    pub binary: BinaryHeader,
    /// Traces in file order.
    pub traces: Vec<Trace>,
}

/// Convert a 32-bit IBM System/360 float (sign, 7-bit base-16 exponent excess 64, 24-bit fraction)
/// to `f32`.
pub fn ibm_to_f32(b: [u8; 4]) -> f32 {
    let sign = if b[0] & 0x80 != 0 { -1.0 } else { 1.0 };
    let exp = (b[0] & 0x7F) as i32 - 64;
    let frac = ((b[1] as u32) << 16) | ((b[2] as u32) << 8) | b[3] as u32;
    if frac == 0 {
        return 0.0;
    }
    (sign * (frac as f64) / 16_777_216.0 * 16f64.powi(exp)) as f32
}

fn bytes_per_sample(format: u16) -> Result<usize, SegyError> {
    match format {
        1 | 2 | 5 => Ok(4),
        3 => Ok(2),
        8 => Ok(1),
        other => Err(SegyError::Unsupported(alloc::format!("format code {other}"))),
    }
}

fn decode(format: u16, bytes: &[u8]) -> Vec<f32> {
    match format {
        1 => bytes.chunks_exact(4).map(|c| ibm_to_f32([c[0], c[1], c[2], c[3]])).collect(),
        2 => bytes
            .chunks_exact(4)
            .map(|c| i32::from_be_bytes([c[0], c[1], c[2], c[3]]) as f32)
            .collect(),
        3 => bytes
            .chunks_exact(2)
            .map(|c| i16::from_be_bytes([c[0], c[1]]) as f32)
            .collect(),
        8 => bytes.iter().map(|&b| b as i8 as f32).collect(),
        _ => bytes
            .chunks_exact(4)
            .map(|c| f32::from_be_bytes([c[0], c[1], c[2], c[3]]))
            .collect(),
    }
}

/// Parse a SEG-Y file.
pub fn parse(input: &[u8]) -> Result<SegY, SegyError> {
    let header_len = TEXT_LEN + BIN_LEN;
    if input.len() < header_len {
        return Err(ParseError::eof(input.len()).into());
    }
    let bin = &input[TEXT_LEN..header_len];
    let be16 = |at: usize| u16::from_be_bytes([bin[at], bin[at + 1]]);
    let binary = BinaryHeader {
        sample_interval_us: be16(16),
        samples_per_trace: be16(20),
        format: be16(24),
        raw: bin.to_vec(),
    };
    let width = bytes_per_sample(binary.format)?;
    let mut file = SegY {
        text: input[..TEXT_LEN].to_vec(),
        binary,
        traces: Vec::new(),
    };
    let samples = file.binary.samples_per_trace as usize;
    let mut pos = header_len;
    while pos < input.len() {
        let header_end = pos + TRACE_HEADER_LEN;
        let data_end = header_end + samples * width;
        if data_end > input.len() {
            return Err(ParseError::eof(input.len()).into());
        }
        let mut header = [0u8; TRACE_HEADER_LEN];
        header.copy_from_slice(&input[pos..header_end]);
        let decoded = decode(file.binary.format, &input[header_end..data_end]);
        file.traces.push(Trace { header, samples: decoded });
        pos = data_end;
    }
    Ok(file)
}

/// Serialise a SEG-Y file. Traces are written as IEEE 32-bit floats; the binary header's format code is
/// set to 5 to match.
pub fn write(file: &SegY) -> Vec<u8> {
    let mut out = Vec::new();
    let mut text = file.text.clone();
    text.resize(TEXT_LEN, b' ');
    out.extend_from_slice(&text);
    let mut bin = file.binary.raw.clone();
    bin.resize(BIN_LEN, 0);
    bin[16..18].copy_from_slice(&file.binary.sample_interval_us.to_be_bytes());
    bin[20..22].copy_from_slice(&file.binary.samples_per_trace.to_be_bytes());
    bin[24..26].copy_from_slice(&5u16.to_be_bytes());
    out.extend_from_slice(&bin);
    for t in &file.traces {
        out.extend_from_slice(&t.header);
        for s in &t.samples {
            out.extend_from_slice(&s.to_be_bytes());
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ibm_float_known_values() {
        // 1.0 in IBM format: exponent 65 (base 16), fraction 0x100000 → 1.0.
        assert_eq!(ibm_to_f32([0x41, 0x10, 0x00, 0x00]), 1.0);
        assert_eq!(ibm_to_f32([0xC1, 0x10, 0x00, 0x00]), -1.0);
        assert_eq!(ibm_to_f32([0, 0, 0, 0]), 0.0);
    }

    #[test]
    fn zero_format_codes_are_unsupported() {
        assert!(bytes_per_sample(7).is_err());
    }
}
