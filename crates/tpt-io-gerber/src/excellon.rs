//! Excellon drill-file parser (through-hole / via drilling).
//!
//! Excellon files are line-oriented. A header (between `M48` and `M95`, or a
//! leading `%`) declares the unit (`INCH`/`METRIC`), zero-suppression (`LZ`/`TZ`),
//! and tool definitions (`T01C0.5`). The body then selects tools (`T01`) and
//! gives hole coordinates (`X00Y00`), optionally as linear slots via `G85`.
//!
//! Coordinate scaling reuses [`crate::coord::CoordinateFormat`] so that leading
//! and trailing zero suppression behave exactly as in Gerber.

use alloc::string::{String, ToString};
use alloc::vec::Vec;

use tpt_io_core::{Encoding, Reader, ValidationSettings, Validator};

use crate::coord::{Axis, CoordinateFormat, Notation, ZeroSuppression};
use crate::error::GerberError;
use crate::model::Unit;

/// A single drilled hole.
#[derive(Debug, Clone, PartialEq)]
pub struct Hole {
    /// Tool (drill size) used.
    pub tool: u32,
    /// X position in the file's unit.
    pub x: f64,
    /// Y position in the file's unit.
    pub y: f64,
}

/// A routed slot (linear drill) between two points.
#[derive(Debug, Clone, PartialEq)]
pub struct Slot {
    /// Tool used.
    pub tool: u32,
    /// Start X.
    pub x0: f64,
    /// Start Y.
    pub y0: f64,
    /// End X.
    pub x1: f64,
    /// End Y.
    pub y1: f64,
}

/// A fully parsed Excellon drill file.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ExcellonFile {
    /// Linear unit.
    pub unit: Unit,
    /// Coordinate format (decimal placement + zero suppression).
    pub format: CoordinateFormat,
    /// Tool number → drill diameter (in the file's unit).
    pub tools: Vec<(u32, f64)>,
    /// Drilled holes in body order.
    pub holes: Vec<Hole>,
    /// Routed slots (from `G85`), if any.
    pub slots: Vec<Slot>,
    /// Whether an `M30`/`M00` end was reached.
    pub end_of_file: bool,
}

fn default_format() -> CoordinateFormat {
    CoordinateFormat {
        zeros: ZeroSuppression::Leading,
        notation: Notation::Absolute,
        x_int: 2,
        x_dec: 4,
        y_int: 2,
        y_dec: 4,
    }
}

/// Parse an Excellon document from a string slice.
pub fn parse_str(input: &str) -> Result<ExcellonFile, GerberError> {
    let mut file =
        ExcellonFile { unit: Unit::Inch, format: default_format(), ..Default::default() };
    let mut cur_tool: u32 = 0;
    let mut cur_x: Option<i64> = None;
    let mut cur_y: Option<i64> = None;

    for raw in input.lines() {
        let line = raw.split(';').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        let upper = line.to_ascii_uppercase();

        if upper.starts_with("M48") {
            continue;
        }
        if upper.starts_with("M95") || line == "%" {
            continue;
        }
        if upper.starts_with("M30") || upper.starts_with("M00") {
            file.end_of_file = true;
            continue;
        }
        if upper.starts_with("G90") {
            file.format.notation = Notation::Absolute;
            continue;
        }
        if upper.starts_with("G91") {
            file.format.notation = Notation::Incremental;
            continue;
        }
        if upper.starts_with("FMAT") || upper.starts_with("VER") {
            continue;
        }
        if upper.starts_with("METRIC") {
            file.unit = Unit::Mm;
            file.format.x_int = 3;
            file.format.x_dec = 3;
            file.format.y_int = 3;
            file.format.y_dec = 3;
            continue;
        }
        if upper.starts_with("INCH") {
            file.unit = Unit::Inch;
            file.format.x_int = 2;
            file.format.x_dec = 4;
            file.format.y_int = 2;
            file.format.y_dec = 4;
            continue;
        }
        if upper.starts_with("G85") {
            // Linear slot: G85X..Y..X..Y..
            let coords = parse_coords(&upper, &file.format, &mut cur_x, &mut cur_y);
            if let (Some((ax, ay)), Some((bx, by))) = (coords.first, coords.second) {
                file.slots.push(Slot { tool: cur_tool, x0: ax, y0: ay, x1: bx, y1: by });
            }
            continue;
        }

        // Tool definition / selection.
        if let Some(rest) = line.strip_prefix('T').or_else(|| line.strip_prefix('t')) {
            let bytes = rest.as_bytes();
            let mut n = 0;
            while n < bytes.len() && bytes[n].is_ascii_digit() {
                n += 1;
            }
            if n == 0 {
                continue;
            }
            let tool_no: u32 = rest[..n]
                .parse()
                .map_err(|_| GerberError::invalid_number(0, rest[..n].to_string()))?;
            let tail = &rest[n..];
            if let Some(cpos) = tail.find('C').or_else(|| tail.find('c')) {
                let diam = tail[cpos + 1..].trim().parse::<f64>().unwrap_or(0.0);
                file.tools.push((tool_no, diam));
            } else {
                cur_tool = tool_no;
            }
            continue;
        }

        // Hole coordinates.
        if upper.contains('X') || upper.contains('Y') {
            let coords = parse_coords(&upper, &file.format, &mut cur_x, &mut cur_y);
            let (x, y) = match (coords.first, coords.second) {
                (Some((a, _)), Some((b, _))) => (a, b),
                _ => continue,
            };
            file.holes.push(Hole { tool: cur_tool, x, y });
        }
    }

    Ok(file)
}

struct CoordPair {
    first: Option<(f64, f64)>,
    second: Option<(f64, f64)>,
}

/// Parse up to two `X..Y..` pairs from a statement, tracking incremental state.
fn parse_coords(
    s: &str,
    fmt: &CoordinateFormat,
    cur_x: &mut Option<i64>,
    cur_y: &mut Option<i64>,
) -> CoordPair {
    let b = s.as_bytes();
    let mut pairs: Vec<(f64, f64)> = Vec::new();
    let mut i = 0;
    let mut pending_x: Option<i64> = None;
    let mut pending_y: Option<i64> = None;

    let mut flush = |px: &mut Option<i64>, py: &mut Option<i64>| {
        if px.is_some() || py.is_some() {
            let rx = match *px {
                Some(v) => apply_raw(fmt, Axis::X, cur_x, v),
                None => cur_x.map(|v| fmt.scale_axis(Axis::X, v)).unwrap_or(0.0),
            };
            let ry = match *py {
                Some(v) => apply_raw(fmt, Axis::Y, cur_y, v),
                None => cur_y.map(|v| fmt.scale_axis(Axis::Y, v)).unwrap_or(0.0),
            };
            pairs.push((rx, ry));
            *px = None;
            *py = None;
        }
    };

    while i < b.len() {
        let c = b[i];
        i += 1;
        match c.to_ascii_uppercase() {
            b'X' => {
                let (v, nk) = read_int(b, i);
                i = nk;
                if let Some(v) = v {
                    flush(&mut pending_x, &mut pending_y);
                    pending_x = Some(v);
                }
            }
            b'Y' => {
                let (v, nk) = read_int(b, i);
                i = nk;
                if let Some(v) = v {
                    flush(&mut pending_x, &mut pending_y);
                    pending_y = Some(v);
                }
            }
            _ => {}
        }
    }
    flush(&mut pending_x, &mut pending_y);

    CoordPair { first: pairs.first().copied(), second: pairs.get(1).copied() }
}

fn apply_raw(fmt: &CoordinateFormat, axis: Axis, cur: &mut Option<i64>, raw: i64) -> f64 {
    let new_raw = match fmt.notation {
        Notation::Absolute => raw,
        Notation::Incremental => cur.unwrap_or(0) + raw,
    };
    *cur = Some(new_raw);
    fmt.scale_axis(axis, new_raw)
}

fn read_int(b: &[u8], mut i: usize) -> (Option<i64>, usize) {
    let mut sign = 1i64;
    if i < b.len() && (b[i] == b'+' || b[i] == b'-') {
        if b[i] == b'-' {
            sign = -1;
        }
        i += 1;
    }
    let start = i;
    while i < b.len() && b[i].is_ascii_digit() {
        i += 1;
    }
    if i == start {
        return (None, i);
    }
    let s = core::str::from_utf8(&b[start..i]).unwrap_or("");
    match s.parse::<i64>() {
        Ok(v) => (Some(sign * v), i),
        Err(_) => (None, i),
    }
}

/// Reader for Excellon files, implementing [`tpt_io_core::Reader`].
#[derive(Debug, Clone, Default)]
pub struct ExcellonReader {
    validation: ValidationSettings,
}

impl ExcellonReader {
    /// Create a reader with default validation (syntactic + semantic).
    pub fn new() -> Self {
        Self { validation: ValidationSettings::new() }
    }

    /// Set the validation configuration.
    pub fn with_validation(mut self, validation: ValidationSettings) -> Self {
        self.validation = validation;
        self
    }
}

impl Reader<ExcellonFile> for ExcellonReader {
    type Error = GerberError;

    fn parse(&self, input: &[u8]) -> Result<ExcellonFile, GerberError> {
        let text = Encoding::detect(input)
            .as_str(input)
            .ok_or(GerberError::Unsupported("non-UTF-8 Excellon input"))?;
        let file = parse_str(text)?;
        self.validation.run(&ExcellonValidator, &file)?;
        Ok(file)
    }
}

/// Syntactic / semantic validation for a parsed [`ExcellonFile`].
#[derive(Debug, Clone, Copy, Default)]
pub struct ExcellonValidator;

impl Validator<ExcellonFile> for ExcellonValidator {
    type Error = GerberError;

    fn validate_syntactic(&self, value: &ExcellonFile) -> Result<(), GerberError> {
        let defined: alloc::collections::BTreeSet<u32> =
            value.tools.iter().map(|(id, _)| *id).collect();
        for h in &value.holes {
            if !defined.contains(&h.tool) && h.tool != 0 {
                return Err(GerberError::MissingField("drill references undefined tool"));
            }
        }
        Ok(())
    }

    fn validate_semantic(&self, _value: &ExcellonFile) -> Result<(), GerberError> {
        Ok(())
    }
}

/// Serialize an [`ExcellonFile`] back to Excellon text (round-trip).
pub fn serialize(file: &ExcellonFile) -> Result<alloc::vec::Vec<u8>, GerberError> {
    use alloc::format;
    let mut out = String::new();
    out.push_str("M48\n");
    match file.unit {
        Unit::Inch => out.push_str(if file.format.zeros == ZeroSuppression::Trailing {
            "INCH,TZ\n"
        } else {
            "INCH,LZ\n"
        }),
        Unit::Mm => out.push_str(if file.format.zeros == ZeroSuppression::Trailing {
            "METRIC,TZ\n"
        } else {
            "METRIC,LZ\n"
        }),
    }
    for (t, d) in &file.tools {
        out.push_str(&format!("T{t:02}C{d}\n"));
    }
    out.push_str("M95\n");
    if file.format.notation == Notation::Incremental {
        out.push_str("G91\n");
    } else {
        out.push_str("G90\n");
    }
    let mut last_tool: Option<u32> = None;
    for h in &file.holes {
        if last_tool != Some(h.tool) {
            out.push_str(&format!("T{:02}\n", h.tool));
            last_tool = Some(h.tool);
        }
        out.push_str(&format!(
            "X{}Y{}\n",
            file.format.encode_axis(Axis::X, h.x),
            file.format.encode_axis(Axis::Y, h.y)
        ));
    }
    out.push_str("M30\n");
    Ok(out.into_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_tools_and_holes() {
        let src = "\
M48
INCH,TZ
T01C0.5
T02C0.8
M95
G90
T01
X00Y00
X100Y100
T02
X200Y200
M30
";
        let f = parse_str(src).unwrap();
        assert_eq!(f.unit, Unit::Inch);
        assert_eq!(f.tools.len(), 2);
        assert_eq!(f.tools[0], (1, 0.5));
        assert_eq!(f.holes.len(), 3);
        assert_eq!(f.holes[0].tool, 1);
        assert!((f.holes[1].x - 0.01).abs() < 1e-9); // 100 / 10000
        assert_eq!(f.holes[2].tool, 2);
    }

    #[test]
    fn reader_runs_validation() {
        let src = "M48\nINCH,TZ\nT01C0.5\nM95\nG90\nT01\nX00Y00\nM30\n";
        let f = ExcellonReader::new().parse(src.as_bytes()).unwrap();
        assert_eq!(f.holes.len(), 1);
    }

    #[test]
    fn metric_round_trips() {
        let src = "M48\nMETRIC,LZ\nT01C0.5\nM95\nG90\nT01\nX500Y500\nM30\n";
        let f = parse_str(src).unwrap();
        assert_eq!(f.unit, Unit::Mm);
        let bytes = serialize(&f).unwrap();
        let reparsed = parse_str(core::str::from_utf8(&bytes).unwrap()).unwrap();
        assert_eq!(f, reparsed);
    }
}
