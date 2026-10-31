//! HEC-RAS geometry keyword parsing.

use alloc::string::{String, ToString};
use alloc::vec::Vec;

use tpt_io_core::ParseError;

use crate::error::HecRasError;

/// A cross section: its river station and its station/elevation points.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct CrossSection {
    /// The river station from the `Type RM` line.
    pub river_station: f64,
    /// `(station, elevation)` points from `#Sta/Elev`, in file order.
    pub points: Vec<(f64, f64)>,
}

/// A river reach: its names, centreline and cross sections.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Reach {
    /// The river name.
    pub river: String,
    /// The reach name.
    pub reach: String,
    /// Centreline points from `Reach XY`, as `(x, y)`.
    pub centerline: Vec<(f64, f64)>,
    /// Cross sections in file order.
    pub cross_sections: Vec<CrossSection>,
}

/// A geometry file: its reaches in file order.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct GeometryFile {
    /// Reaches in file order.
    pub reaches: Vec<Reach>,
}

/// A stream of whitespace- and comma-separated numbers spanning lines.
struct Numbers<'a> {
    lines: core::str::Lines<'a>,
    pending: Vec<&'a str>,
    offset: usize,
}

impl<'a> Numbers<'a> {
    fn take(&mut self, n: usize) -> Result<Vec<f64>, HecRasError> {
        let mut out = Vec::with_capacity(n);
        while out.len() < n {
            if self.pending.is_empty() {
                let line = self.lines.next().ok_or_else(|| {
                    HecRasError::Invalid("keyword count runs past the end of the file".to_string())
                })?;
                self.offset += line.len() + 1;
                self.pending = line
                    .split(|c: char| c.is_whitespace() || c == ',')
                    .filter(|t| !t.is_empty())
                    .collect();
                self.pending.reverse();
                continue;
            }
            let tok = self.pending.pop().unwrap_or("");
            let v = tok
                .parse::<f64>()
                .map_err(|_| ParseError::custom(self.offset, "expected a number"))?;
            out.push(v);
        }
        Ok(out)
    }
}

/// Parse a HEC-RAS geometry file.
pub fn parse(input: &[u8]) -> Result<GeometryFile, HecRasError> {
    let text = core::str::from_utf8(input)
        .map_err(|e| ParseError::custom(e.valid_up_to(), "file is not valid UTF-8"))?;
    let mut g = GeometryFile::default();
    let mut numbers = Numbers { lines: text.lines(), pending: Vec::new(), offset: 0 };
    while let Some(line) = numbers.lines.next() {
        numbers.offset += line.len() + 1;
        let t = line.trim();
        if let Some(rest) = t.strip_prefix("River Reach=") {
            let (river, reach) = rest.split_once(',').unwrap_or((rest, ""));
            g.reaches.push(Reach {
                river: river.trim().to_string(),
                reach: reach.trim().to_string(),
                ..Reach::default()
            });
        } else if let Some(rest) = t.strip_prefix("Reach XY=") {
            let count = count_of(rest)?;
            let values = numbers.take(count * 2)?;
            let reach = current(&mut g)?;
            reach.centerline = values.chunks_exact(2).map(|c| (c[0], c[1])).collect();
        } else if t.starts_with("Type RM") {
            let station = t
                .split_once('=')
                .and_then(|(_, rhs)| rhs.split(',').nth(1))
                .and_then(|s| s.trim().parse::<f64>().ok())
                .ok_or_else(|| {
                    HecRasError::Invalid("cross section has no river station".to_string())
                })?;
            let reach = current(&mut g)?;
            reach.cross_sections.push(CrossSection { river_station: station, points: Vec::new() });
        } else if let Some(rest) = t.strip_prefix("#Sta/Elev=") {
            let count = count_of(rest)?;
            let values = numbers.take(count * 2)?;
            let reach = current(&mut g)?;
            let section = reach.cross_sections.last_mut().ok_or_else(|| {
                HecRasError::Invalid("#Sta/Elev outside a cross section".to_string())
            })?;
            section.points = values.chunks_exact(2).map(|c| (c[0], c[1])).collect();
        }
    }
    Ok(g)
}

fn count_of(rest: &str) -> Result<usize, HecRasError> {
    rest.trim()
        .split(|c: char| c == ',' || c.is_whitespace())
        .next()
        .and_then(|t| t.parse::<usize>().ok())
        .ok_or_else(|| HecRasError::Invalid("keyword has no count".to_string()))
}

fn current(g: &mut GeometryFile) -> Result<&mut Reach, HecRasError> {
    g.reaches
        .last_mut()
        .ok_or_else(|| HecRasError::Invalid("keyword before any River Reach line".to_string()))
}
