//! SP3 precise orbit reader.

use alloc::string::{String, ToString};
use alloc::vec::Vec;

use tpt_io_core::ParseError;

use crate::error::GnssError;

/// A satellite's position at an epoch: kilometres in the ECEF frame, and the clock offset in microseconds
/// when given (missing values, `999999`, become `None`).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Sp3Position {
    /// Satellite id, e.g. `G01`.
    pub id: String,
    /// Position `[x, y, z]` in kilometres.
    pub position_km: [f64; 3],
    /// Clock offset in microseconds.
    pub clock_us: Option<f64>,
}

/// One SP3 epoch: its time text and the satellite positions at that time.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Sp3Epoch {
    /// The epoch text after `*`, e.g. `2024 01 01 00 00 0.00000000`.
    pub time: String,
    /// Positions in file order.
    pub positions: Vec<Sp3Position>,
}

/// An SP3 file.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Sp3 {
    /// The version letter from the first line (`c` or `d`).
    pub version: char,
    /// Epochs in file order.
    pub epochs: Vec<Sp3Epoch>,
}

fn number(s: &str, at: usize) -> Result<f64, GnssError> {
    s.parse::<f64>().map_err(|_| ParseError::custom(at, "expected a number").into())
}

/// Parse an SP3 file (position records, `P` lines).
pub fn parse_sp3(input: &[u8]) -> Result<Sp3, GnssError> {
    let text = core::str::from_utf8(input)
        .map_err(|e| ParseError::custom(e.valid_up_to(), "file is not valid UTF-8"))?;
    let mut sp3 = Sp3::default();
    let mut offset = 0usize;
    for line in text.lines() {
        let at = offset;
        offset += line.len() + 1;
        if sp3.version == '\0' && line.starts_with('#') {
            sp3.version = line.chars().nth(1).unwrap_or('c');
            continue;
        }
        if let Some(rest) = line.strip_prefix('*') {
            sp3.epochs.push(Sp3Epoch { time: rest.trim().to_string(), positions: Vec::new() });
            continue;
        }
        if let Some(rest) = line.strip_prefix('P') {
            let epoch = sp3
                .epochs
                .last_mut()
                .ok_or_else(|| GnssError::Invalid("position before any epoch".to_string()))?;
            let cols: Vec<&str> = rest.split_whitespace().collect();
            if cols.len() < 4 {
                return Err(ParseError::custom(
                    at,
                    "position line needs an id and three coordinates",
                )
                .into());
            }
            let id = cols[0].to_string();
            let position_km = [number(cols[1], at)?, number(cols[2], at)?, number(cols[3], at)?];
            let clock_us = match cols.get(4).map(|c| number(c, at)) {
                Some(Ok(v)) if v < 999_999.0 => Some(v),
                Some(Err(e)) => return Err(e),
                _ => None,
            };
            epoch.positions.push(Sp3Position { id, position_km, clock_us });
        }
    }
    if sp3.version == '\0' {
        return Err(GnssError::Invalid("missing SP3 version line".to_string()));
    }
    Ok(sp3)
}
