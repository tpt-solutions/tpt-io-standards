//! SPICE `.raw` simulation output, as written by ngspice and LTspice.
//!
//! A raw file has a text header (title, date, plot name, flags, variable and point counts, and the variable
//! list), followed by the data. `Binary:` introduces the data as 64-bit little-endian floats, point by point, with
//! one value per variable. `Values:` introduces the data as text: one line per point, starting with its index.
//! Only real-valued plots are read and written; complex plots (`Flags: complex`) are rejected.
//!
//! ```
//! use tpt_io_spice::raw::{self, RawFile, Variable};
//!
//! let plot = RawFile {
//!     title: "rc filter".to_string(),
//!     date: "Mon Jan  1 00:00:00 2026".to_string(),
//!     plotname: "Transient Analysis".to_string(),
//!     variables: vec![
//!         Variable { index: 0, name: "time".to_string(), kind: "time".to_string() },
//!         Variable { index: 1, name: "v(out)".to_string(), kind: "voltage".to_string() },
//!     ],
//!     columns: vec![vec![0.0, 1e-3], vec![1.0, 0.5]],
//! };
//! let bytes = raw::write(&plot).unwrap();
//! assert_eq!(raw::parse(&bytes).unwrap(), plot);
//! ```
//!
//! Spec access: the raw file format is documented by the ngspice project and by Analog Devices (LTspice), and is
//! widely re-implemented. No specification text is copied into this crate.

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::fmt::Write as _;

use tpt_io_core::ParseError;

use crate::error::SpiceError;

/// One variable (column) of a plot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Variable {
    /// The variable index, as written in the header.
    pub index: usize,
    /// The variable name, such as `time` or `v(out)`.
    pub name: String,
    /// The variable kind, such as `time`, `voltage` or `current`.
    pub kind: String,
}

/// A raw plot: header fields, the variable list, and one column of values per variable.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct RawFile {
    /// The `Title:` header value.
    pub title: String,
    /// The `Date:` header value.
    pub date: String,
    /// The `Plotname:` header value, such as `Transient Analysis`.
    pub plotname: String,
    /// The variables, in column order.
    pub variables: Vec<Variable>,
    /// One column per variable, each holding one value per point.
    pub columns: Vec<Vec<f64>>,
}

impl RawFile {
    /// The number of points in the plot.
    pub fn points(&self) -> usize {
        self.columns.first().map_or(0, |c| c.len())
    }

    /// The column of values for the variable with the given name.
    pub fn column(&self, name: &str) -> Option<&[f64]> {
        let i = self.variables.iter().position(|v| v.name == name)?;
        self.columns.get(i).map(|c| c.as_slice())
    }
}

/// Parse a raw file, binary or ASCII.
pub fn parse(input: &[u8]) -> Result<RawFile, SpiceError> {
    let mut raw = RawFile::default();
    let mut nvars = None;
    let mut npoints = None;
    let mut in_variables = false;
    let mut marker = None;
    let mut offset = 0;
    while offset < input.len() {
        let end =
            input[offset..].iter().position(|&b| b == b'\n').map_or(input.len(), |p| offset + p);
        let next = (end + 1).min(input.len());
        let line = core::str::from_utf8(&input[offset..end])
            .map_err(|_| ParseError::custom(offset, "raw header is not text"))?
            .trim_end_matches('\r');
        if line == "Binary:" || line == "Values:" {
            marker = Some((line == "Binary:", next));
            break;
        }
        if in_variables && line.starts_with('\t') {
            raw.variables.push(variable(line, offset)?);
        } else {
            in_variables = false;
            if let Some(v) = line.strip_prefix("Title:") {
                raw.title = v.trim().to_string();
            } else if let Some(v) = line.strip_prefix("Date:") {
                raw.date = v.trim().to_string();
            } else if let Some(v) = line.strip_prefix("Plotname:") {
                raw.plotname = v.trim().to_string();
            } else if let Some(v) = line.strip_prefix("Flags:") {
                if v.split_whitespace().any(|f| f == "complex") {
                    return Err(SpiceError::Unsupported("complex raw plots"));
                }
            } else if let Some(v) = line.strip_prefix("No. Variables:") {
                nvars = Some(count(v, offset)?);
            } else if let Some(v) = line.strip_prefix("No. Points:") {
                npoints = Some(count(v, offset)?);
            } else if line == "Variables:" {
                in_variables = true;
            }
        }
        offset = next;
    }
    let (binary, data_at) = marker.ok_or_else(|| {
        ParseError::custom(input.len(), "raw file has no Binary: or Values: section")
    })?;
    let nvars = nvars.ok_or_else(|| ParseError::custom(0, "raw header has no No. Variables"))?;
    let npoints = npoints.ok_or_else(|| ParseError::custom(0, "raw header has no No. Points"))?;
    if raw.variables.len() != nvars {
        return Err(ParseError::custom(0, "variable list does not match No. Variables").into());
    }
    raw.columns = alloc::vec![Vec::new(); nvars];
    if binary {
        let need = npoints
            .checked_mul(nvars)
            .and_then(|n| n.checked_mul(8))
            .ok_or_else(|| ParseError::custom(data_at, "raw point count overflows"))?;
        let data = input.get(data_at..data_at + need).ok_or_else(|| ParseError::eof(data_at))?;
        for p in 0..npoints {
            for (v, column) in raw.columns.iter_mut().enumerate() {
                let at = (p * nvars + v) * 8;
                let mut b = [0u8; 8];
                b.copy_from_slice(&data[at..at + 8]);
                column.push(f64::from_le_bytes(b));
            }
        }
    } else {
        let text = core::str::from_utf8(&input[data_at..])
            .map_err(|_| ParseError::custom(data_at, "raw ASCII values are not text"))?;
        let mut tokens = text.split_whitespace();
        for _ in 0..npoints {
            // Each point starts with its index on the same line as its first value.
            tokens.next().ok_or_else(|| ParseError::eof(data_at))?;
            for column in raw.columns.iter_mut() {
                let t = tokens.next().ok_or_else(|| ParseError::eof(data_at))?;
                let value = t
                    .parse::<f64>()
                    .map_err(|_| ParseError::custom(data_at, "malformed raw value"))?;
                column.push(value);
            }
        }
    }
    Ok(raw)
}

/// Write a real-valued raw plot in binary form.
pub fn write(raw: &RawFile) -> Result<Vec<u8>, SpiceError> {
    if raw.columns.len() != raw.variables.len() {
        return Err(ParseError::custom(0, "column count does not match variable count").into());
    }
    let points = raw.points();
    if raw.columns.iter().any(|c| c.len() != points) {
        return Err(ParseError::custom(0, "columns have different lengths").into());
    }
    let mut header = String::new();
    let _ = writeln!(header, "Title: {}", raw.title.replace(['\r', '\n'], " "));
    let _ = writeln!(header, "Date: {}", raw.date.replace(['\r', '\n'], " "));
    let _ = writeln!(header, "Plotname: {}", raw.plotname.replace(['\r', '\n'], " "));
    header.push_str("Flags: real\n");
    let _ = writeln!(header, "No. Variables: {}", raw.variables.len());
    let _ = writeln!(header, "No. Points: {points}");
    header.push_str("Variables:\n");
    for v in &raw.variables {
        let _ = writeln!(header, "\t{}\t{}\t{}", v.index, v.name, v.kind);
    }
    header.push_str("Binary:\n");
    let mut out = header.into_bytes();
    for p in 0..points {
        for column in &raw.columns {
            out.extend_from_slice(&column[p].to_le_bytes());
        }
    }
    Ok(out)
}

fn variable(line: &str, offset: usize) -> Result<Variable, SpiceError> {
    let mut fields = line.split_whitespace();
    let index = fields
        .next()
        .and_then(|t| t.parse::<usize>().ok())
        .ok_or_else(|| ParseError::custom(offset, "variable line has no index"))?;
    let name = fields
        .next()
        .ok_or_else(|| ParseError::custom(offset, "variable line has no name"))?
        .to_string();
    let kind = fields.next().unwrap_or("").to_string();
    Ok(Variable { index, name, kind })
}

fn count(text: &str, offset: usize) -> Result<usize, SpiceError> {
    text.trim()
        .parse::<usize>()
        .map_err(|_| ParseError::custom(offset, "malformed count in raw header").into())
}
