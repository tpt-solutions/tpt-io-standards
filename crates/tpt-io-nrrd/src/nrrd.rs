//! NRRD header, inline data, sample decoding, and writing.

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::fmt::Write as _;

use tpt_io_core::ParseError;

use crate::error::NrrdError;

/// Decodes one sample from its bytes; the flag is true for big-endian data.
type Decode = fn(&[u8], bool) -> f64;

/// A parsed NRRD file: its magic line, header fields in file order, and the raw data bytes.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Nrrd {
    /// The magic line, e.g. `NRRD0004`.
    pub magic: String,
    /// Header fields as `(name, value)` in file order. Names are lower-cased; `key:=value` pairs keep
    /// their `:=` marker in the name.
    pub fields: Vec<(String, String)>,
    /// The data bytes after the header, as stored (ASCII data remains text).
    pub data: Vec<u8>,
}

impl Nrrd {
    /// The value of a header field (case-insensitive).
    pub fn field(&self, name: &str) -> Option<&str> {
        self.fields.iter().find(|(k, _)| k == name).map(|(_, v)| v.as_str())
    }

    /// The `sizes` field as axis lengths.
    pub fn sizes(&self) -> Option<Vec<usize>> {
        self.field("sizes")?.split_whitespace().map(|t| t.parse::<usize>().ok()).collect()
    }

    /// The number of samples implied by the sizes.
    pub fn sample_count(&self) -> Option<usize> {
        self.sizes().map(|s| s.iter().product())
    }

    /// Decode the data into `f64` samples according to the `type` and `endian` fields.
    /// Supports `uchar`, `char`, `short`/`int16`, `ushort`/`uint16`, `int`/`int32`, `uint`/`uint32`,
    /// `float`, and `double` for raw data, and whitespace-separated numbers for ASCII data.
    pub fn values(&self) -> Result<Vec<f64>, NrrdError> {
        let encoding = self.field("encoding").unwrap_or("raw");
        let ty = self.field("type").unwrap_or("uchar");
        if encoding == "ascii" || encoding == "text" || encoding == "txt" {
            let text = core::str::from_utf8(&self.data)
                .map_err(|_| NrrdError::Invalid("ASCII data is not text".to_string()))?;
            return text
                .split_whitespace()
                .map(|t| {
                    t.parse::<f64>().map_err(|_| NrrdError::Invalid("bad ASCII sample".to_string()))
                })
                .collect();
        }
        if encoding != "raw" {
            return Err(NrrdError::Unsupported(encoding.to_string()));
        }
        let big = self.field("endian") == Some("big");
        let (width, decode): (usize, Decode) = match ty {
            "uchar" | "unsigned char" | "uint8" | "uint8_t" => (1, |b, _| b[0] as f64),
            "char" | "int8" | "int8_t" => (1, |b, _| b[0] as i8 as f64),
            "short" | "int16" | "int16_t" => (2, |b, big| {
                let a = [b[0], b[1]];
                (if big { i16::from_be_bytes(a) } else { i16::from_le_bytes(a) }) as f64
            }),
            "ushort" | "uint16" | "uint16_t" | "unsigned short" => (2, |b, big| {
                let a = [b[0], b[1]];
                (if big { u16::from_be_bytes(a) } else { u16::from_le_bytes(a) }) as f64
            }),
            "int" | "int32" | "int32_t" => (4, |b, big| {
                let a = [b[0], b[1], b[2], b[3]];
                (if big { i32::from_be_bytes(a) } else { i32::from_le_bytes(a) }) as f64
            }),
            "uint" | "uint32" | "uint32_t" | "unsigned int" => (4, |b, big| {
                let a = [b[0], b[1], b[2], b[3]];
                (if big { u32::from_be_bytes(a) } else { u32::from_le_bytes(a) }) as f64
            }),
            "float" => (4, |b, big| {
                let a = [b[0], b[1], b[2], b[3]];
                (if big { f32::from_be_bytes(a) } else { f32::from_le_bytes(a) }) as f64
            }),
            "double" => (8, |b, big| {
                let mut a = [0u8; 8];
                a.copy_from_slice(&b[..8]);
                if big {
                    f64::from_be_bytes(a)
                } else {
                    f64::from_le_bytes(a)
                }
            }),
            other => return Err(NrrdError::Unsupported(other.to_string())),
        };
        let count = self.sample_count().unwrap_or(0);
        if self.data.len() < count * width {
            return Err(NrrdError::Invalid("data is shorter than its sizes".to_string()));
        }
        Ok(self.data[..count * width].chunks_exact(width).map(|c| decode(c, big)).collect())
    }
}

/// Parse an NRRD file.
pub fn parse(input: &[u8]) -> Result<Nrrd, NrrdError> {
    let line_at = |pos: usize| -> Option<(&[u8], usize)> {
        let rest = input.get(pos..)?;
        let end = rest.iter().position(|&b| b == b'\n')?;
        Some((&rest[..end], pos + end + 1))
    };
    let (magic, next) =
        line_at(0).ok_or_else(|| ParseError::custom(0, "missing NRRD magic line"))?;
    let magic = core::str::from_utf8(magic)
        .map_err(|_| ParseError::custom(0, "magic is not text"))?
        .to_string();
    if !magic.starts_with("NRRD") {
        return Err(ParseError::custom(0, "missing NRRD magic").into());
    }
    let mut pos = next;
    let mut fields = Vec::new();
    loop {
        let Some((line, next)) = line_at(pos) else {
            return Err(NrrdError::Invalid("header is not terminated by a blank line".to_string()));
        };
        let text = core::str::from_utf8(line)
            .map_err(|_| ParseError::custom(pos, "header is not text"))?;
        pos = next;
        if text.is_empty() {
            break;
        }
        if text.starts_with('#') {
            continue;
        }
        let (name, value) = if let Some((k, v)) = text.split_once(":=") {
            (alloc::format!("{}:=", k.trim().to_ascii_lowercase()), v.trim())
        } else if let Some((k, v)) = text.split_once(':') {
            (k.trim().to_ascii_lowercase(), v.trim())
        } else {
            return Err(ParseError::custom(pos, "header line has no ':'").into());
        };
        fields.push((name, value.to_string()));
    }
    let mut nrrd = Nrrd { magic, fields, data: input[pos..].to_vec() };
    if nrrd.field("data file").is_some() || nrrd.field("datafile").is_some() {
        return Err(NrrdError::Unsupported("external data files".to_string()));
    }
    if let Some(enc) = nrrd.field("encoding").map(|e| e.to_string()) {
        if enc == "gzip" || enc == "gz" || enc == "bzip2" || enc == "bz2" {
            return Err(NrrdError::Unsupported(enc));
        }
    }
    nrrd.fields.shrink_to_fit();
    Ok(nrrd)
}

/// Serialise an NRRD file: the magic, the header fields, a blank line, and the data.
pub fn write(n: &Nrrd) -> Vec<u8> {
    let mut out = String::new();
    let _ = writeln!(out, "{}", n.magic);
    for (k, v) in &n.fields {
        if let Some(name) = k.strip_suffix(":=") {
            let _ = writeln!(out, "{name}:={v}");
        } else {
            let _ = writeln!(out, "{k}: {v}");
        }
    }
    out.push('\n');
    let mut bytes = out.into_bytes();
    bytes.extend_from_slice(&n.data);
    bytes
}
