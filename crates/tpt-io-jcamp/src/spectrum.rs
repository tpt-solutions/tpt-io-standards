//! JCAMP-DX labelled records, XY data, and writing.

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::fmt::Write as _;

use tpt_io_core::ParseError;

use crate::error::JcampError;

/// A parsed spectrum: its labelled header records and its (x, y) points.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Spectrum {
    /// Header records as `(label, value)` in file order. Labels are without `##` and are upper-cased.
    pub header: Vec<(String, String)>,
    /// Points as `(x, y)` with the X and Y factors applied.
    pub points: Vec<(f64, f64)>,
}

impl Spectrum {
    /// The value of a header label (case-insensitive), e.g. `TITLE`.
    pub fn label(&self, name: &str) -> Option<&str> {
        self.header.iter().find(|(k, _)| k == name).map(|(_, v)| v.as_str())
    }

    /// The `TITLE` label.
    pub fn title(&self) -> Option<&str> {
        self.label("TITLE")
    }
}

fn number(tok: &str, at: usize) -> Result<f64, JcampError> {
    let fixed = tok.replace(['D', 'd'], "E");
    fixed.parse::<f64>().map_err(|_| ParseError::custom(at, "expected a number").into())
}

/// Parse a JCAMP-DX file.
pub fn parse(input: &[u8]) -> Result<Spectrum, JcampError> {
    let text = core::str::from_utf8(input)
        .map_err(|e| ParseError::custom(e.valid_up_to(), "file is not valid UTF-8"))?;
    let mut header: Vec<(String, String)> = Vec::new();
    // Each label starts a record; continuation lines (no `##`) extend the previous value.
    let mut records: Vec<(String, String, usize)> = Vec::new();
    let mut offset = 0usize;
    for line in text.lines() {
        let at = offset;
        offset += line.len() + 1;
        if let Some(rest) = line.strip_prefix("##") {
            let (label, value) = match rest.split_once('=') {
                Some((l, v)) => (l.trim().to_ascii_uppercase(), v.trim().to_string()),
                None => (rest.trim().to_ascii_uppercase(), String::new()),
            };
            records.push((label, value, at));
        } else if let Some(last) = records.last_mut() {
            if !line.trim().is_empty() {
                last.1.push('\n');
                last.1.push_str(line.trim());
            }
        } else if !line.trim().is_empty() {
            return Err(ParseError::custom(at, "text before the first ## label").into());
        }
    }

    let factor = |name: &str| -> Result<f64, JcampError> {
        match records.iter().find(|(l, _, _)| l == name) {
            Some((_, v, at)) => number(v.trim(), *at),
            None => Ok(1.0),
        }
    };
    let xf = factor("XFACTOR")?;
    let yf = factor("YFACTOR")?;

    let mut points = Vec::new();
    let mut found_data = false;
    for (label, value, at) in &records {
        if label == "XYPOINTS" || label == "XYDATA" {
            found_data = true;
            let form = value.split_whitespace().next().unwrap_or("").to_ascii_uppercase();
            // The block's header is on the first line; the data follows on the lines after it.
            let body = value.split_once('\n').map(|(_, b)| b).unwrap_or("");
            if label == "XYPOINTS" {
                let nums = body
                    .split(|c: char| c.is_whitespace() || c == ',' || c == ';')
                    .filter(|t| !t.is_empty())
                    .map(|t| number(t, *at))
                    .collect::<Result<Vec<f64>, _>>()?;
                if nums.len() % 2 != 0 {
                    return Err(JcampError::Invalid("XYPOINTS needs x, y pairs".into()));
                }
                points.extend(nums.chunks_exact(2).map(|c| (c[0] * xf, c[1] * yf)));
            } else {
                // Compressed values use letters and the '@' and '%' characters; plain values use only
                // digits, signs, points and the E or D exponent marker.
                let compressed = body.chars().any(|c| {
                    (c.is_ascii_alphabetic() && !matches!(c, 'E' | 'e' | 'D' | 'd'))
                        || c == '@'
                        || c == '%'
                });
                if compressed
                    || form.contains("SQZ")
                    || form.contains("DIF")
                    || form.contains("DUP")
                {
                    return Err(JcampError::Unsupported(form));
                }
                // X++(Y..Y): each line gives an X value then Y values at successive X steps.
                let mut lines_with_x: Vec<Vec<f64>> = Vec::new();
                for row in body.lines() {
                    let nums = row
                        .split(|c: char| c.is_whitespace() || c == ',')
                        .filter(|t| !t.is_empty())
                        .map(|t| number(t, *at))
                        .collect::<Result<Vec<f64>, _>>()?;
                    if !nums.is_empty() {
                        lines_with_x.push(nums);
                    }
                }
                let step = factor("DELTAX").ok().filter(|d| *d != 1.0).unwrap_or(0.0);
                for row in lines_with_x {
                    let x0 = row[0] * xf;
                    for (k, y) in row[1..].iter().enumerate() {
                        let x = if step != 0.0 { x0 + k as f64 * step } else { x0 + k as f64 * xf };
                        points.push((x, y * yf));
                    }
                }
            }
        }
    }
    if !found_data {
        return Err(JcampError::Invalid("no XYDATA or XYPOINTS block".into()));
    }
    for (label, value, _) in records {
        if label != "XYPOINTS" && label != "XYDATA" && label != "END" {
            header.push((label, value));
        }
    }
    Ok(Spectrum { header, points })
}

/// Serialise a spectrum: its header records, then the points as an `XYPOINTS` block.
pub fn write(s: &Spectrum) -> Vec<u8> {
    let mut out = String::new();
    for (label, value) in &s.header {
        if value.contains('\n') {
            let _ = writeln!(out, "##{label}={}", value.lines().next().unwrap_or(""));
            for line in value.lines().skip(1) {
                let _ = writeln!(out, "{line}");
            }
        } else {
            let _ = writeln!(out, "##{label}={value}");
        }
    }
    // The reader multiplies stored values by the factors in the header, so store values divided by them.
    let factor = |name: &str| {
        s.label(name).and_then(|v| v.parse::<f64>().ok()).filter(|f| *f != 0.0).unwrap_or(1.0)
    };
    let (xf, yf) = (factor("XFACTOR"), factor("YFACTOR"));
    out.push_str("##XYPOINTS=(XY..XY)\n");
    for (x, y) in &s.points {
        let _ = writeln!(out, "{} {}", x / xf, y / yf);
    }
    out.push_str("##END=\n");
    out.into_bytes()
}
