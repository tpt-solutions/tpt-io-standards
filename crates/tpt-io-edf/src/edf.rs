//! EDF/BDF header, signal definitions, samples, and writing.

use alloc::string::{String, ToString};
use alloc::vec::Vec;

use tpt_io_core::ParseError;

use crate::error::EdfError;

/// One signal's definition from the signal header block.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Signal {
    /// The label, e.g. `EEG Fpz-Cz`.
    pub label: String,
    /// The transducer description.
    pub transducer: String,
    /// The physical dimension, e.g. `uV`.
    pub physical_dim: String,
    /// Lower physical limit.
    pub physical_min: f64,
    /// Upper physical limit.
    pub physical_max: f64,
    /// Lower digital (raw) limit.
    pub digital_min: i32,
    /// Upper digital (raw) limit.
    pub digital_max: i32,
    /// The prefiltering description.
    pub prefiltering: String,
    /// Samples per data record for this signal.
    pub samples_per_record: usize,
}

impl Signal {
    /// The physical value of a raw sample, scaled linearly between the digital and physical limits.
    pub fn to_physical(&self, raw: i32) -> f64 {
        let span = f64::from(self.digital_max) - f64::from(self.digital_min);
        if span == 0.0 {
            return self.physical_min;
        }
        (f64::from(raw) - f64::from(self.digital_min)) * (self.physical_max - self.physical_min)
            / span
            + self.physical_min
    }
}

/// A recording: the header fields, the signals, and the raw samples per signal.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Edf {
    /// `true` for BDF (24-bit samples), `false` for EDF.
    pub bdf: bool,
    /// The patient identification field.
    pub patient: String,
    /// The recording identification field.
    pub recording: String,
    /// Start date as written (`dd.mm.yy`).
    pub start_date: String,
    /// Start time as written (`hh.mm.ss`).
    pub start_time: String,
    /// Duration of one data record in seconds.
    pub record_duration: f64,
    /// Number of data records.
    pub records: usize,
    /// Signal definitions in order.
    pub signals: Vec<Signal>,
    /// Raw samples per signal, concatenated over all records.
    pub samples: Vec<Vec<i32>>,
}

impl Edf {
    /// The physical values of one signal (index into `signals`).
    pub fn physical(&self, signal: usize) -> Vec<f64> {
        match (self.signals.get(signal), self.samples.get(signal)) {
            (Some(s), Some(raw)) => raw.iter().map(|r| s.to_physical(*r)).collect(),
            _ => Vec::new(),
        }
    }
}

fn field(bytes: &[u8], at: usize, len: usize) -> Result<&str, ParseError> {
    let s = bytes.get(at..at + len).ok_or_else(|| ParseError::eof(at))?;
    core::str::from_utf8(s)
        .map(|t| t.trim_end())
        .map_err(|_| ParseError::custom(at, "header field is not ASCII"))
}

fn number<T: core::str::FromStr>(s: &str, at: usize) -> Result<T, EdfError> {
    s.trim().parse::<T>().map_err(|_| ParseError::custom(at, "header field is not a number").into())
}

/// Parse an EDF or BDF file.
pub fn parse(input: &[u8]) -> Result<Edf, EdfError> {
    if input.len() < 256 {
        return Err(ParseError::eof(input.len()).into());
    }
    let bdf = input[0] == 0xFF;
    let mut e = Edf {
        bdf,
        patient: field(input, 8, 80)?.to_string(),
        recording: field(input, 88, 80)?.to_string(),
        start_date: field(input, 168, 8)?.to_string(),
        start_time: field(input, 176, 8)?.to_string(),
        ..Edf::default()
    };
    let header_bytes: usize = number(field(input, 184, 8)?, 184)?;
    e.records = number(field(input, 236, 8)?, 236)?;
    e.record_duration = number(field(input, 244, 8)?, 244)?;
    let ns: usize = number(field(input, 252, 4)?, 252)?;
    if header_bytes != 256 * (ns + 1) {
        return Err(EdfError::Invalid("header size does not match the signal count".into()));
    }
    let mut at = 256;
    let block = |at: usize, width: usize| -> Result<Vec<&str>, ParseError> {
        (0..ns).map(|i| field(input, at + i * width, width)).collect()
    };
    let labels = block(256, 16)?;
    let transducers = block(256 + 16 * ns, 80)?;
    let dims = block(256 + 96 * ns, 8)?;
    let pmins = block(256 + 104 * ns, 8)?;
    let pmaxs = block(256 + 112 * ns, 8)?;
    let dmins = block(256 + 120 * ns, 8)?;
    let dmaxs = block(256 + 128 * ns, 8)?;
    let prefilters = block(256 + 136 * ns, 80)?;
    let samples_fields = block(256 + 216 * ns, 8)?;
    for i in 0..ns {
        e.signals.push(Signal {
            label: labels[i].to_string(),
            transducer: transducers[i].to_string(),
            physical_dim: dims[i].to_string(),
            physical_min: number(pmins[i], 0)?,
            physical_max: number(pmaxs[i], 0)?,
            digital_min: number(dmins[i], 0)?,
            digital_max: number(dmaxs[i], 0)?,
            prefiltering: prefilters[i].to_string(),
            samples_per_record: number(samples_fields[i], 0)?,
        });
    }
    at = at.max(256 * (ns + 1));
    let width = if bdf { 3 } else { 2 };
    let per_record: usize = e.signals.iter().map(|s| s.samples_per_record).sum();
    let needed = at + e.records * per_record * width;
    if input.len() < needed {
        return Err(EdfError::Invalid("file is shorter than its records declare".into()));
    }
    e.samples = alloc::vec![Vec::new(); ns];
    let mut pos = at;
    for _ in 0..e.records {
        for (i, s) in e.signals.iter().enumerate() {
            for _ in 0..s.samples_per_record {
                let raw = if bdf {
                    let b = &input[pos..pos + 3];
                    let v = i32::from(b[0]) | (i32::from(b[1]) << 8) | (i32::from(b[2]) << 16);
                    if v & 0x80_0000 != 0 {
                        v - 0x100_0000
                    } else {
                        v
                    }
                } else {
                    i32::from(i16::from_le_bytes([input[pos], input[pos + 1]]))
                };
                e.samples[i].push(raw);
                pos += width;
            }
        }
    }
    Ok(e)
}

fn put(out: &mut Vec<u8>, text: &str, width: usize) {
    let mut s: Vec<u8> = text.bytes().take(width).collect();
    s.resize(width, b' ');
    out.extend_from_slice(&s);
}

/// Serialise an EDF (16-bit) file. Fails for BDF, which this crate does not write.
pub fn write(e: &Edf) -> Result<Vec<u8>, EdfError> {
    if e.bdf {
        return Err(EdfError::Unsupported("writing BDF".into()));
    }
    let ns = e.signals.len();
    if e.samples.len() != ns {
        return Err(EdfError::Invalid("sample arrays do not match the signal count".into()));
    }
    let mut out = Vec::new();
    put(&mut out, "0", 8);
    put(&mut out, &e.patient, 80);
    put(&mut out, &e.recording, 80);
    put(&mut out, &e.start_date, 8);
    put(&mut out, &e.start_time, 8);
    put(&mut out, &(256 * (ns + 1)).to_string(), 8);
    put(&mut out, "", 44);
    put(&mut out, &e.records.to_string(), 8);
    put(&mut out, &alloc::format!("{}", e.record_duration), 8);
    put(&mut out, &ns.to_string(), 4);
    for s in &e.signals {
        put(&mut out, &s.label, 16);
    }
    for s in &e.signals {
        put(&mut out, &s.transducer, 80);
    }
    for s in &e.signals {
        put(&mut out, &s.physical_dim, 8);
    }
    for s in &e.signals {
        put(&mut out, &alloc::format!("{}", s.physical_min), 8);
    }
    for s in &e.signals {
        put(&mut out, &alloc::format!("{}", s.physical_max), 8);
    }
    for s in &e.signals {
        put(&mut out, &s.digital_min.to_string(), 8);
    }
    for s in &e.signals {
        put(&mut out, &s.digital_max.to_string(), 8);
    }
    for s in &e.signals {
        put(&mut out, &s.prefiltering, 80);
    }
    for s in &e.signals {
        put(&mut out, &s.samples_per_record.to_string(), 8);
    }
    for _ in &e.signals {
        put(&mut out, "", 32);
    }
    for r in 0..e.records {
        for (i, s) in e.signals.iter().enumerate() {
            for k in 0..s.samples_per_record {
                let v = e.samples[i].get(r * s.samples_per_record + k).copied().unwrap_or(0);
                let v = i16::try_from(v)
                    .map_err(|_| EdfError::Invalid("sample does not fit in 16 bits".into()))?;
                out.extend_from_slice(&v.to_le_bytes());
            }
        }
    }
    Ok(out)
}
