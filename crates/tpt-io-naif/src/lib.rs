//! NAIF DAF and SPK kernels: the binary ephemeris files of the SPICE toolkit (`.bsp`).
//!
//! A DAF file is a sequence of 1024-byte records. The file record names the format and gives the number of double
//! and integer components in each segment summary. Summary records list the segments, and the name record that
//! follows each one holds their names. An SPK kernel stores the position and velocity of a body as Chebyshev
//! polynomials over time intervals. [`Daf::parse`] reads the summaries, [`Daf::segment`] reads a segment's words,
//! and [`spk::state`] evaluates a segment of type 2 or 3 at an epoch.
//!
//! Spec access: the DAF and SPK formats are documented in NAIF's SPICE required reading, which is publicly
//! available. No specification text is copied into this crate. Only little- and big-endian IEEE files are read.

#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]

#[cfg(not(any(feature = "alloc", feature = "std")))]
compile_error!("tpt-io-naif requires the `alloc` or `std` feature: its summaries own their names.");

#[cfg(any(feature = "alloc", feature = "std"))]
#[allow(unused_imports)]
#[macro_use]
extern crate alloc;

pub mod spk;

use alloc::string::{String, ToString};
use alloc::vec::Vec;

use tpt_io_core::ParseError;

/// Bytes per DAF record.
const RECORD: usize = 1024;
/// Bytes per double-precision word.
const WORD: usize = 8;

/// Errors produced while reading a DAF or SPK kernel.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum NaifError {
    /// A truncated or malformed record, with its byte offset.
    Core(ParseError),
    /// The kernel breaks a rule (not a DAF file, a bad summary, a bad segment layout).
    Invalid(String),
    /// A feature the reader does not implement, such as an unsupported binary format or SPK segment type.
    Unsupported(&'static str),
    /// No segment covers the requested body and epoch.
    NoSegment,
}

impl From<ParseError> for NaifError {
    fn from(e: ParseError) -> Self {
        NaifError::Core(e)
    }
}

impl core::fmt::Display for NaifError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            NaifError::Core(e) => write!(f, "{e}"),
            NaifError::Invalid(what) => write!(f, "invalid NAIF kernel: {what}"),
            NaifError::Unsupported(what) => write!(f, "NAIF unsupported: {what}"),
            NaifError::NoSegment => write!(f, "no segment covers the requested body and epoch"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for NaifError {}

/// The byte order of the file's binary data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Endian {
    /// `LTL-IEEE`.
    Little,
    /// `BIG-IEEE`.
    Big,
}

/// One segment summary: its time coverage, the body and centre it describes, its frame and data type, and where its
/// data sits in the file.
#[derive(Debug, Clone, PartialEq)]
pub struct Summary {
    /// The first epoch covered, in seconds past J2000 (TDB for SPK).
    pub start_et: f64,
    /// The last epoch covered.
    pub end_et: f64,
    /// The NAIF id of the body the segment describes.
    pub target: i32,
    /// The NAIF id of the centre the state is relative to.
    pub center: i32,
    /// The NAIF frame id, such as 1 for J2000.
    pub frame: i32,
    /// The SPK data type, such as 2 (Chebyshev position) or 3 (Chebyshev position and velocity).
    pub data_type: i32,
    /// The first word of the segment data (1-based, as stored).
    pub begin: usize,
    /// The last word of the segment data (1-based, inclusive, as stored).
    pub end: usize,
    /// The segment name from the name record.
    pub name: String,
}

/// A parsed DAF kernel, borrowing the file bytes. Segment data is read on demand.
#[derive(Debug, Clone)]
pub struct Daf<'a> {
    /// The file identifier, such as `DAF/SPK`.
    pub id: String,
    /// The byte order of the binary data.
    pub endian: Endian,
    /// The number of double components in each summary.
    pub nd: usize,
    /// The number of integer components in each summary.
    pub ni: usize,
    /// Every segment summary, in file order.
    pub summaries: Vec<Summary>,
    data: &'a [u8],
}

fn int_at(data: &[u8], at: usize, endian: Endian) -> Result<i32, NaifError> {
    let s = data.get(at..at + 4).ok_or_else(|| ParseError::eof(at))?;
    let b = [s[0], s[1], s[2], s[3]];
    Ok(match endian {
        Endian::Little => i32::from_le_bytes(b),
        Endian::Big => i32::from_be_bytes(b),
    })
}

fn double_at(data: &[u8], at: usize, endian: Endian) -> Result<f64, NaifError> {
    let s = data.get(at..at + WORD).ok_or_else(|| ParseError::eof(at))?;
    let mut b = [0u8; WORD];
    b.copy_from_slice(s);
    Ok(match endian {
        Endian::Little => f64::from_le_bytes(b),
        Endian::Big => f64::from_be_bytes(b),
    })
}

impl<'a> Daf<'a> {
    /// Read the file record and every summary. Summaries come from the chain of summary records, starting at the
    /// file record's forward pointer.
    pub fn parse(data: &'a [u8]) -> Result<Self, NaifError> {
        if data.len() < RECORD {
            return Err(ParseError::eof(data.len()).into());
        }
        let id = String::from_utf8_lossy(&data[0..8]).trim_end().to_string();
        if !id.starts_with("DAF/") {
            return Err(NaifError::Invalid(
                "file does not begin with a DAF identifier".to_string(),
            ));
        }
        let format = String::from_utf8_lossy(&data[88..96]).trim_end().to_string();
        let endian = match format.as_str() {
            "LTL-IEEE" => Endian::Little,
            "BIG-IEEE" => Endian::Big,
            _ => {
                return Err(NaifError::Unsupported("only LTL-IEEE and BIG-IEEE DAF files are read"))
            }
        };
        let nd = usize::try_from(int_at(data, 8, endian)?).unwrap_or(0);
        let ni = usize::try_from(int_at(data, 12, endian)?).unwrap_or(0);
        if nd < 2 || ni < 6 || nd > 124 || ni > 250 {
            return Err(NaifError::Invalid("summary sizes are out of range".to_string()));
        }
        let summary_words = nd + ni.div_ceil(2);
        let mut summaries = Vec::new();
        let mut record = usize::try_from(int_at(data, 76, endian)?).unwrap_or(0);
        let mut visited = 0usize;
        while record != 0 {
            visited += 1;
            if visited > data.len() / RECORD {
                return Err(NaifError::Invalid("summary record chain loops".to_string()));
            }
            let base = (record - 1) * RECORD;
            let next = double_at(data, base, endian)? as usize;
            let count = double_at(data, base + 16, endian)? as usize;
            let first = summaries.len();
            for i in 0..count {
                let at = base + 24 + i * summary_words * WORD;
                let ints = at + nd * WORD;
                summaries.push(Summary {
                    start_et: double_at(data, at, endian)?,
                    end_et: double_at(data, at + WORD, endian)?,
                    target: int_at(data, ints, endian)?,
                    center: int_at(data, ints + 4, endian)?,
                    frame: int_at(data, ints + 8, endian)?,
                    data_type: int_at(data, ints + 12, endian)?,
                    begin: usize::try_from(int_at(data, ints + 16, endian)?).unwrap_or(0),
                    end: usize::try_from(int_at(data, ints + 20, endian)?).unwrap_or(0),
                    name: String::new(),
                });
            }
            // The name record follows the summary record, one name per summary.
            let names = base + RECORD;
            for (i, s) in summaries[first..].iter_mut().enumerate() {
                let at = names + i * summary_words * WORD;
                if let Some(raw) = data.get(at..at + summary_words * WORD) {
                    s.name = String::from_utf8_lossy(raw)
                        .trim_matches(|c: char| c == '\0' || c.is_whitespace())
                        .to_string();
                }
            }
            record = next;
        }
        Ok(Daf { id, endian, nd, ni, summaries, data })
    }

    /// The `index`-th word of the file (1-based), as a double.
    pub fn word(&self, index: usize) -> Result<f64, NaifError> {
        let at = index
            .checked_sub(1)
            .and_then(|i| i.checked_mul(WORD))
            .ok_or_else(|| ParseError::custom(0, "word index must be at least 1"))?;
        double_at(self.data, at, self.endian)
    }

    /// The double-precision words of a segment, from its first word to its last.
    pub fn segment(&self, s: &Summary) -> Result<Vec<f64>, NaifError> {
        if s.begin == 0 || s.end < s.begin {
            return Err(NaifError::Invalid("segment word range is malformed".to_string()));
        }
        let count = s.end - s.begin + 1;
        let start = (s.begin - 1) * WORD;
        let bytes =
            self.data.get(start..start + count * WORD).ok_or_else(|| ParseError::eof(start))?;
        let mut out = Vec::with_capacity(count);
        for chunk in bytes.chunks_exact(WORD) {
            let mut b = [0u8; WORD];
            b.copy_from_slice(chunk);
            out.push(match self.endian {
                Endian::Little => f64::from_le_bytes(b),
                Endian::Big => f64::from_be_bytes(b),
            });
        }
        Ok(out)
    }
}
