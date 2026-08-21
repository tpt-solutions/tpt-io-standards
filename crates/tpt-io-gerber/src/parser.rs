//! Gerber (RS-274X / X2 / X3) parser.
//!
//! A Gerber file is a stream of *parameter blocks* (`%…%`) and *standard
//! statements* (terminated by `*`): parameter blocks carry the format, unit,
//! polarity, aperture definitions, macros, and X2/X3 attributes; statements
//! carry graphics operations (coordinate draws, flashes, arcs, regions). The
//! parser resolves coordinates against the active [`CoordinateFormat`] and
//! emits a structured [`GerberFile`].
//!
//! Parsing is allocation-based (the result owns its strings/vectors) and works
//! over a `&[u8]` input after UTF-8 detection via `tpt_io_core::Encoding`.

use alloc::string::{String, ToString};
use alloc::vec::Vec;

use tpt_io_core::{Encoding, Reader, ValidationSettings, Validator};

use crate::aperture::{parse_add, parse_am};
use crate::attributes::Attributes;
use crate::coord::{Axis, CoordinateFormat, Notation};
use crate::error::GerberError;
use crate::model::{GerberFile, GraphicObject, Interpolation, ObjectKind, Polarity, Unit};

/// Parse a Gerber document from a string slice.
pub fn parse_str(input: &str) -> Result<GerberFile, GerberError> {
    let bytes = input.as_bytes();
    let mut tokens: Vec<(usize, Token)> = Vec::new();
    let mut i = 0usize;
    let mut stmt_start: Option<usize> = None;

    while i < bytes.len() {
        let c = bytes[i];
        if c == b'%' {
            // Flush any in-progress statement up to this block.
            if let Some(start) = stmt_start.take() {
                tokens.push((start, Token::Statement(input[start..i].to_string())));
            }
            i += 1;
            let mut j = i;
            while j < bytes.len() && bytes[j] != b'%' {
                j += 1;
            }
            let content = input[i..j.min(bytes.len())].trim_end_matches('*').to_string();
            tokens.push((i, Token::Block(content)));
            if j >= bytes.len() {
                break;
            }
            i = j + 1;
        } else if c == b'*' {
            let start = stmt_start.take().unwrap_or(i);
            tokens.push((start, Token::Statement(input[start..i].to_string())));
            i += 1;
        } else {
            if stmt_start.is_none() {
                stmt_start = Some(i);
            }
            i += 1;
        }
    }
    if let Some(start) = stmt_start.take() {
        tokens.push((start, Token::Statement(input[start..bytes.len()].to_string())));
    }

    let mut p = Parser::default();
    for (offset, tok) in tokens {
        match tok {
            Token::Block(b) => p.parse_block(offset, &b)?,
            Token::Statement(s) => p.parse_statement(offset, &s)?,
        }
    }
    if let Some(pts) = p.region.take() {
        p.emit_region(pts);
    }
    Ok(p.file)
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Token {
    Block(String),
    Statement(String),
}

#[derive(Default)]
struct Parser {
    file: GerberFile,
    cur_x: Option<i64>,
    cur_y: Option<i64>,
    cur_ap: Option<u32>,
    interp: Interpolation,
    region_started: bool,
    region: Option<Vec<(f64, f64)>>,
    pending_obj_attr: Option<Attributes>,
    pending_ap_attr: Option<Attributes>,
    last_op: u8,
}

impl Parser {
    fn world(&self, axis: Axis) -> f64 {
        let raw = match axis {
            Axis::X => self.cur_x,
            Axis::Y => self.cur_y,
        }
        .unwrap_or(0);
        self.file.format.scale_axis(axis, raw)
    }

    fn update_raw(&mut self, axis: Axis, raw: i64) {
        let slot = match axis {
            Axis::X => &mut self.cur_x,
            Axis::Y => &mut self.cur_y,
        };
        *slot = Some(match self.file.format.notation {
            Notation::Absolute => raw,
            Notation::Incremental => slot.unwrap_or(0) + raw,
        });
    }

    /// Resolve a coordinate token: update the raw counter if present, return the
    /// resulting world position.
    fn coord_value(&mut self, axis: Axis, raw: Option<i64>) -> f64 {
        if let Some(v) = raw {
            self.update_raw(axis, v);
        }
        self.world(axis)
    }

    fn emit(&mut self, kind: ObjectKind) {
        let attr = self.pending_obj_attr.take().unwrap_or_default();
        self.file.objects.push(GraphicObject { kind, aperture: self.cur_ap, attributes: attr });
    }

    fn emit_region(&mut self, points: Vec<(f64, f64)>) {
        let attr = self.pending_obj_attr.take().unwrap_or_default();
        self.file.objects.push(GraphicObject {
            kind: ObjectKind::Region { points },
            aperture: None,
            attributes: attr,
        });
    }

    fn parse_block(&mut self, _offset: usize, block: &str) -> Result<(), GerberError> {
        let b = block.as_bytes();
        if b.is_empty() {
            return Ok(());
        }
        match b[0].to_ascii_uppercase() {
            b'F' => {
                let fmt = CoordinateFormat::parse(block)?;
                self.file.format = fmt;
            }
            b'M' => {
                if b.len() >= 3 && b[1].eq_ignore_ascii_case(&b'O') {
                    match b[2].to_ascii_uppercase() {
                        b'I' => self.file.unit = Unit::Inch,
                        b'M' => self.file.unit = Unit::Mm,
                        _ => {}
                    }
                }
            }
            b'L' => {
                if b.len() >= 3 && b[1].eq_ignore_ascii_case(&b'P') {
                    match b[2].to_ascii_uppercase() {
                        b'D' => self.file.polarity = Polarity::Positive,
                        b'C' => self.file.polarity = Polarity::Negative,
                        _ => {}
                    }
                }
            }
            b'A' => {
                if b.len() >= 2 && b[1].eq_ignore_ascii_case(&b'D') {
                    let (id, ap) = parse_add(block)?;
                    if let Some(attr) = self.pending_ap_attr.take() {
                        self.file.aperture_attributes.push((id, attr));
                    }
                    self.file.apertures.push((id, ap));
                } else if b.len() >= 2 && b[1].eq_ignore_ascii_case(&b'M') {
                    let m = parse_am(block)?;
                    let name = m.name.clone();
                    self.file.aperture_macros.push((name, m));
                }
            }
            b'T' => self.parse_t_attribute(block)?,
            _ => {}
        }
        Ok(())
    }

    fn parse_t_attribute(&mut self, block: &str) -> Result<(), GerberError> {
        let kind = block.get(..2).unwrap_or("").to_ascii_uppercase();
        let rest = block.get(2..).unwrap_or("");
        match kind.as_str() {
            "TF" => {
                if let Some((name, vals)) = split_attr(rest) {
                    let is_polarity = name.eq_ignore_ascii_case("FilePolarity");
                    self.file.file_attributes.insert(name, vals);
                    if is_polarity {
                        if let Some(v) =
                            self.file.file_attributes.get("FilePolarity").and_then(|v| v.first())
                        {
                            self.file.polarity = if v.eq_ignore_ascii_case("Negative") {
                                Polarity::Negative
                            } else {
                                Polarity::Positive
                            };
                        }
                    }
                }
            }
            "TA" => {
                if let Some((name, vals)) = split_attr(rest) {
                    let mut a = Attributes::new();
                    a.insert(name, vals);
                    self.pending_ap_attr = Some(a);
                }
            }
            "TO" => {
                if let Some((name, vals)) = split_attr(rest) {
                    let mut a = Attributes::new();
                    a.insert(name, vals);
                    self.pending_obj_attr = Some(a);
                }
            }
            "TD" => {
                self.pending_obj_attr = None;
                self.pending_ap_attr = None;
            }
            "TN" => {
                self.file.file_attributes.insert("NP".to_string(), alloc::vec!["N".to_string()]);
            }
            "TP" => {
                self.file.file_attributes.insert("NP".to_string(), alloc::vec!["P".to_string()]);
            }
            _ => {}
        }
        Ok(())
    }

    fn parse_statement(&mut self, _offset: usize, stmt: &str) -> Result<(), GerberError> {
        let s = stmt.trim();
        if s.is_empty() {
            return Ok(());
        }
        if s.to_ascii_uppercase().starts_with("G04") {
            return Ok(()); // comment
        }
        let compact: String = s.chars().filter(|c| !c.is_whitespace()).collect();
        let cb = compact.as_bytes();

        let mut x: Option<i64> = None;
        let mut y: Option<i64> = None;
        let mut i_v: Option<i64> = None;
        let mut j_v: Option<i64> = None;
        let mut d: Option<i64> = None;
        let mut g: Option<i64> = None;
        let mut m: Option<i64> = None;

        let mut k = 0usize;
        while k < cb.len() {
            let c = cb[k];
            k += 1;
            let (num, nk) = read_int(cb, k);
            k = nk;
            match c.to_ascii_uppercase() {
                b'X' => x = num,
                b'Y' => y = num,
                b'I' => i_v = num,
                b'J' => j_v = num,
                b'D' => d = num,
                b'G' => g = num,
                b'M' => m = num,
                _ => {}
            }
        }

        // G codes (interpolation / region / quadrant / unit).
        if let Some(g) = g {
            match g {
                1 => self.interp = Interpolation::Linear,
                2 => self.interp = Interpolation::CircularCw,
                3 => self.interp = Interpolation::CircularCcw,
                36 => {
                    let sx = self.world(Axis::X);
                    let sy = self.world(Axis::Y);
                    self.region = Some(alloc::vec![(sx, sy)]);
                    self.region_started = true;
                }
                37 => {
                    if let Some(pts) = self.region.take() {
                        self.emit_region(pts);
                    }
                    self.region_started = false;
                }
                70 => self.file.unit = Unit::Inch,
                71 => self.file.unit = Unit::Mm,
                _ => {}
            }
        }

        if let Some(d) = d {
            if d >= 10 {
                self.cur_ap = Some(d as u32);
            }
        }

        if let Some(m) = m {
            if m == 2 || m == 0 {
                self.file.end_of_file = true;
            }
        }

        // Determine the operation for any coordinates in this statement.
        let op = match d {
            Some(1) => Some(1u8),
            Some(2) => Some(2u8),
            Some(3) => Some(3u8),
            _ => None,
        };
        let op = op.or(if self.last_op != 0 { Some(self.last_op) } else { None });

        if x.is_some() || y.is_some() {
            if let Some(op) = op {
                let old = (self.world(Axis::X), self.world(Axis::Y));
                let new = (self.coord_value(Axis::X, x), self.coord_value(Axis::Y, y));
                let ij = match self.interp {
                    Interpolation::Linear => None,
                    _ => {
                        let i = i_v.map(|v| self.file.format.scale_axis(Axis::X, v)).unwrap_or(0.0);
                        let j = j_v.map(|v| self.file.format.scale_axis(Axis::Y, v)).unwrap_or(0.0);
                        Some((i, j))
                    }
                };
                self.apply_op(op, old, new, ij)?;
            }
        }
        if let Some(d) = d {
            if (1..=3).contains(&d) {
                self.last_op = d as u8;
            }
        }
        Ok(())
    }

    fn apply_op(
        &mut self,
        op: u8,
        old: (f64, f64),
        new: (f64, f64),
        ij: Option<(f64, f64)>,
    ) -> Result<(), GerberError> {
        let (old_x, old_y) = old;
        let (nx, ny) = new;
        if let Some(region) = self.region.as_mut() {
            if op == 2 {
                if region.len() == 1 {
                    region[0] = (nx, ny);
                }
            } else {
                region.push((nx, ny));
            }
            return Ok(());
        }

        match op {
            2 => { /* move only */ }
            3 => self.emit(ObjectKind::Flash { x: nx, y: ny }),
            1 => {
                let kind = match self.interp {
                    Interpolation::Linear => {
                        ObjectKind::Line { x0: old_x, y0: old_y, x1: nx, y1: ny }
                    }
                    _ => {
                        let (i, j) = ij.unwrap_or((0.0, 0.0));
                        ObjectKind::Arc {
                            x0: old_x,
                            y0: old_y,
                            x1: nx,
                            y1: ny,
                            i,
                            j,
                            ccw: self.interp == Interpolation::CircularCcw,
                        }
                    }
                };
                self.emit(kind);
            }
            _ => {}
        }
        Ok(())
    }
}

/// Split an attribute body (after the `TF`/`TA`/`TO` prefix) into `(name, values)`.
///
/// The body looks like `.FileFunction,Copper,L1,Top` — a leading dot, then the
/// attribute name, then comma-separated values (which may be empty).
fn split_attr(rest: &str) -> Option<(String, Vec<String>)> {
    let body = rest.strip_prefix('.').unwrap_or(rest);
    if body.is_empty() {
        return None;
    }
    match body.split_once(',') {
        Some((name, vals)) => {
            let values = if vals.is_empty() {
                Vec::new()
            } else {
                vals.split(',').map(|s| s.to_string()).collect()
            };
            Some((name.to_string(), values))
        }
        None => Some((body.to_string(), Vec::new())),
    }
}

/// Read an optional sign followed by digits, returning `(value, new_index)`.
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

/// Reader for Gerber files, implementing [`tpt_io_core::Reader`] and
/// [`tpt_io_core::Writer`] (round-trip).
#[derive(Debug, Clone, Default)]
pub struct GerberReader {
    validation: ValidationSettings,
}

impl GerberReader {
    /// Create a reader with default validation (syntactic + semantic).
    pub fn new() -> Self {
        Self { validation: ValidationSettings::new() }
    }

    /// Set the validation configuration (e.g. disable semantic checks).
    pub fn with_validation(mut self, validation: ValidationSettings) -> Self {
        self.validation = validation;
        self
    }
}

impl Reader<GerberFile> for GerberReader {
    type Error = GerberError;

    fn parse(&self, input: &[u8]) -> Result<GerberFile, GerberError> {
        let text = Encoding::detect(input)
            .as_str(input)
            .ok_or(GerberError::Unsupported("non-UTF-8 Gerber input"))?;
        let file = parse_str(text)?;
        self.validation.run(&GerberValidator, &file)?;
        Ok(file)
    }
}

impl tpt_io_core::Writer<GerberFile> for GerberReader {
    type Error = GerberError;

    fn serialize(&self, value: &GerberFile) -> Result<Vec<u8>, GerberError> {
        crate::writer::serialize(value)
    }
}

/// Syntactic / semantic validation for a parsed [`GerberFile`].
#[derive(Debug, Clone, Copy, Default)]
pub struct GerberValidator;

impl Validator<GerberFile> for GerberValidator {
    type Error = GerberError;

    fn validate_syntactic(&self, value: &GerberFile) -> Result<(), GerberError> {
        let defined: alloc::collections::BTreeSet<u32> =
            value.apertures.iter().map(|(id, _)| *id).collect();
        for obj in &value.objects {
            if let Some(ap) = obj.aperture {
                if !defined.contains(&ap) {
                    return Err(GerberError::MissingField("referenced aperture is undefined"));
                }
            }
            if let ObjectKind::Arc { i, j, .. } = &obj.kind {
                if *i == 0.0 && *j == 0.0 {
                    return Err(GerberError::MissingField("arc center (I/J) required"));
                }
            }
        }
        Ok(())
    }

    fn validate_semantic(&self, value: &GerberFile) -> Result<(), GerberError> {
        for (_, ap) in &value.apertures {
            if let (Some(h), Some(w)) = (ap.hole_diameter(), ap.nominal_width()) {
                if h >= w {
                    return Err(GerberError::custom(format!(
                        "aperture hole diameter {h} must be smaller than width {w}"
                    )));
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tpt_io_core::Reader;

    const SAMPLE: &str = "\
%FSLAX36Y36*%
%MOIN*%
%LPD*%
%ADD10C,0.5*%
G75*
G01*
D10*
X0Y0D02*
X100000Y0D01*
X100000Y100000D01*
X0Y100000D01*
X0Y0D01*
M02*
";

    #[test]
    fn parses_coordinates_and_aperture() {
        let f = parse_str(SAMPLE).unwrap();
        assert_eq!(f.unit, Unit::Inch);
        assert_eq!(f.polarity, Polarity::Positive);
        assert_eq!(f.apertures.len(), 1);
        assert_eq!(f.apertures[0].0, 10);
        assert_eq!(f.objects.len(), 4);
        assert!(f.end_of_file);
    }

    #[test]
    fn draws_are_lines_with_correct_endpoints() {
        let f = parse_str(SAMPLE).unwrap();
        let lines: Vec<&ObjectKind> = f.objects.iter().map(|o| &o.kind).collect();
        match lines[0] {
            ObjectKind::Line { x0, y0, x1, y1 } => {
                assert!((x0 - 0.0).abs() < 1e-6 && (y0 - 0.0).abs() < 1e-6);
                assert!((x1 - 0.1).abs() < 1e-6 && (y1 - 0.0).abs() < 1e-6);
            }
            other => panic!("expected line, got {other:?}"),
        }
    }

    #[test]
    fn reader_runs_validation() {
        let f = GerberReader::new().parse(SAMPLE.as_bytes()).unwrap();
        assert_eq!(f.objects.len(), 4);
    }

    #[test]
    fn parses_x2_attributes_and_polarity() {
        let src = "\
%FSLAX36Y36*%
%MOMM*%
%TF.FilePolarity,Negative*%
%TF.FileFunction,Copper,L1,Top*%
%TO.N,GND*%
%ADD10C,0.5*%
D10*
X0Y0D03*
M02*
";
        let f = parse_str(src).unwrap();
        assert_eq!(f.polarity, Polarity::Negative);
        assert_eq!(f.file_attributes.file_function(), Some("Copper"));
        assert!(!f.objects.is_empty());
        assert_eq!(f.objects[0].attributes.net_names(), vec!["GND"]);
    }

    #[test]
    fn parses_arcs() {
        let src = "\
%FSLAX36Y36*%
%MOMM*%
%ADD10C,0.5*%
G02*
D10*
X0Y0D02*
X100000Y0D01*
X0Y100000D01*
M02*
";
        let f = parse_str(src).unwrap();
        assert!(f.objects.iter().any(|o| matches!(o.kind, ObjectKind::Arc { ccw: false, .. })));
    }

    #[test]
    fn parses_region() {
        let src = "\
%FSLAX36Y36*%
%MOMM*%
%ADD10C,0.5*%
D10*
G36*
X0Y0D02*
X100000Y0D01*
X100000Y100000D01*
X0Y100000D01*
G37*
M02*
";
        let f = parse_str(src).unwrap();
        assert!(f.objects.iter().any(|o| matches!(o.kind, ObjectKind::Region { .. })));
    }
}
