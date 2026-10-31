//! RINEX 3 observation file reader.

use alloc::string::{String, ToString};
use alloc::vec::Vec;

use tpt_io_core::ParseError;

use crate::error::GnssError;

/// One satellite's observations at an epoch. `values` follows the order of the observation types for its
/// system; a missing observation is `None`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Satellite {
    /// Satellite id, e.g. `G01` (system letter and PRN).
    pub id: String,
    /// Observation values in header order.
    pub values: Vec<Option<f64>>,
}

/// One epoch: its time fields, flag, and satellites.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Epoch {
    /// Year, month, day, hour, minute, and second (with fraction).
    pub time: (i32, u8, u8, u8, u8, f64),
    /// Epoch flag (0 is OK).
    pub flag: u8,
    /// The satellites observed at this epoch.
    pub satellites: Vec<Satellite>,
}

/// A RINEX 3 observation file.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct RinexObs {
    /// The RINEX version, e.g. `3.04`.
    pub version: String,
    /// The `MARKER NAME`, if present.
    pub marker: Option<String>,
    /// The approximate position `APPROX POSITION XYZ`, if present.
    pub approx_position: Option<[f64; 3]>,
    /// `(system letter, observation codes)` from the `SYS / # / OBS TYPES` records.
    pub obs_types: Vec<(char, Vec<String>)>,
    /// Epochs in file order.
    pub epochs: Vec<Epoch>,
}

fn field(line: &str, from: usize, to: usize) -> &str {
    line.get(from..to.min(line.len())).unwrap_or("").trim()
}

fn number(s: &str, at: usize) -> Result<f64, GnssError> {
    s.parse::<f64>().map_err(|_| ParseError::custom(at, "expected a number").into())
}

/// Parse a RINEX 3 observation file.
pub fn parse_rinex_obs(input: &[u8]) -> Result<RinexObs, GnssError> {
    let text = core::str::from_utf8(input)
        .map_err(|e| ParseError::custom(e.valid_up_to(), "file is not valid UTF-8"))?;
    let mut obs = RinexObs::default();
    let lines = text.lines();
    let mut offset = 0usize;
    let mut in_header = true;
    let mut ended_header = false;
    // Obs types may continue across lines; track the system whose list is open.
    let mut open_system: Option<char> = None;
    for line in lines {
        let at = offset;
        offset += line.len() + 1;
        if in_header {
            let label = line.get(60..).unwrap_or("").trim();
            match label {
                "RINEX VERSION / TYPE" => obs.version = field(line, 0, 9).to_string(),
                "MARKER NAME" => obs.marker = Some(field(line, 0, 60).to_string()),
                "APPROX POSITION XYZ" => {
                    let c: Vec<&str> = line.split_whitespace().take(3).collect();
                    if c.len() == 3 {
                        obs.approx_position =
                            Some([number(c[0], at)?, number(c[1], at)?, number(c[2], at)?]);
                    }
                }
                "SYS / # / OBS TYPES" => {
                    let body = line.get(..60).unwrap_or("");
                    let first = body.chars().next().filter(|c| *c != ' ');
                    let system = first.or(open_system).ok_or_else(|| {
                        GnssError::Invalid("OBS TYPES continuation without a system".to_string())
                    })?;
                    open_system = Some(system);
                    let codes = body.get(6..).unwrap_or("").split_whitespace();
                    let entry = match obs.obs_types.iter_mut().find(|(s, _)| *s == system) {
                        Some(e) => e,
                        None => {
                            obs.obs_types.push((system, Vec::new()));
                            obs.obs_types.last_mut().expect("just pushed")
                        }
                    };
                    entry.1.extend(codes.map(|c| c.to_string()));
                }
                "END OF HEADER" => {
                    in_header = false;
                    ended_header = true;
                }
                _ => {}
            }
            continue;
        }
        if let Some(rest) = line.strip_prefix('>') {
            let c: Vec<&str> = rest.split_whitespace().collect();
            if c.len() < 6 {
                return Err(ParseError::custom(at, "epoch line needs a date and time").into());
            }
            let year = c[0].parse::<i32>().map_err(|_| ParseError::custom(at, "bad year"))?;
            let month = c[1].parse::<u8>().map_err(|_| ParseError::custom(at, "bad month"))?;
            let day = c[2].parse::<u8>().map_err(|_| ParseError::custom(at, "bad day"))?;
            let hour = c[3].parse::<u8>().map_err(|_| ParseError::custom(at, "bad hour"))?;
            let minute = c[4].parse::<u8>().map_err(|_| ParseError::custom(at, "bad minute"))?;
            let second = number(c[5], at)?;
            let flag = c.get(6).and_then(|f| f.parse::<u8>().ok()).unwrap_or(0);
            obs.epochs.push(Epoch {
                time: (year, month, day, hour, minute, second),
                flag,
                satellites: Vec::new(),
            });
            continue;
        }
        if line.trim().is_empty() {
            continue;
        }
        let epoch = obs
            .epochs
            .last_mut()
            .ok_or_else(|| GnssError::Invalid("observation line before any epoch".to_string()))?;
        let id = field(line, 0, 3).to_string();
        let system = id.chars().next().unwrap_or(' ');
        let codes =
            obs.obs_types.iter().find(|(s, _)| *s == system).map(|(_, c)| c.len()).unwrap_or(0);
        let mut values = Vec::with_capacity(codes);
        for k in 0..codes {
            let start = 3 + 16 * k;
            let v = field(line, start, start + 14);
            values.push(if v.is_empty() { None } else { Some(number(v, at + start)?) });
        }
        epoch.satellites.push(Satellite { id, values });
    }
    if !ended_header {
        return Err(GnssError::Invalid("no END OF HEADER line".to_string()));
    }
    Ok(obs)
}
