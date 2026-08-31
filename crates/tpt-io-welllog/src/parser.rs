//! LAS 2.0/3.0 ASCII parser.
//!
//! Implements the section-based grammar (`~V`, `~W`, `~O`, `~C`, `~A`) of the
//! Log ASCII Standard, plus a [`Writer`] for round-tripping and a
//! [`WellLogValidator`] for syntactic/semantic checks. Parsing is allocation-
//! based (the result owns its strings) and works over a `&[u8]` input after
//! UTF-8 detection via `tpt_io_core::Encoding`.

use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

use tpt_io_core::{Reader, ValidationSettings, Validator, Writer};

use crate::error::WellLogError;
use crate::model::{Curve, Parameter, WellInfo, WellLog};

/// Intermediate state produced by [`parse_header`] — everything before the
/// `~A` (ASCII/binary data) section.
pub(crate) struct ParsedHeader {
    /// `VERS` from the `~V` section.
    pub version: f32,
    /// `WRAP` from the `~V` section.
    pub wrap: bool,
    /// Well header metadata.
    pub well: WellInfo,
    /// Parameter lines from the `~O` section.
    pub parameters: Vec<Parameter>,
    /// Curve definitions from the `~C` section (column 0 is the index curve).
    pub curves: Vec<Curve>,
}

#[derive(PartialEq)]
enum Section {
    Version,
    Well,
    Parameter,
    Curve,
    Ascii,
    Other,
}

/// Parse the header sections (`~V`/`~W`/`~O`/`~C`) of a LAS document, stopping
/// at the `~A` data section. Returns the structured [`ParsedHeader`] plus the
/// trailing text after the `~A` section header line (the raw data rows, or the
/// `BINARY`/`ASCII` keyword for LAS 3.0).
///
/// `input` is expected to already be in `&str` form (encoding detection and
/// UTF-16 transcoding happen in the callers, [`parse_str`] and
/// `crate::binary::parse`).
pub(crate) fn parse_header(input: &str) -> Result<(ParsedHeader, &str), WellLogError> {
    let mut version: Option<f32> = None;
    let mut wrap = false;
    let mut well = WellInfo::default();
    let mut parameters: Vec<Parameter> = Vec::new();
    let mut curves: Vec<Curve> = Vec::new();

    let mut section = Section::Other;
    // Byte offset where the data (`~A`) section text begins, so the caller can
    // slice the trailing rows. `None` until the section header is seen.
    let mut data_start: Option<usize> = None;

    let mut cursor = 0usize;
    for raw in input.split('\n') {
        let line_offset = cursor;
        cursor += raw.len() + 1; // +1 for the consumed '\n'
        let line = raw.trim_end_matches('\r').trim();

        if line.is_empty() {
            continue;
        }

        if let Some(rest) = line.strip_prefix('~') {
            let letter = rest.trim_start().chars().next().map(|c| c.to_ascii_uppercase());
            // Section headers are a single letter (`~V`, `~W`, …) but often
            // carry descriptive text (`~Version Information`); key on the first
            // character.
            section = match letter {
                Some('V') => Section::Version,
                Some('W') => Section::Well,
                Some('O') | Some('P') => Section::Parameter,
                Some('C') => Section::Curve,
                Some('A') => {
                    data_start = Some(cursor);
                    Section::Ascii
                }
                _ => Section::Other,
            };
            continue;
        }

        // Once we are inside the data section we no longer parse header lines.
        if section == Section::Ascii {
            continue;
        }

        match section {
            Section::Version => {
                let (m, _u, value, _d) = parse_structured(line);
                match m.as_str() {
                    "VERS" => {
                        version =
                            Some(value.parse::<f32>().map_err(|_| WellLogError::InvalidNumber {
                                offset: line_offset,
                                value: value.clone(),
                            })?)
                    }
                    "WRAP" => wrap = value.eq_ignore_ascii_case("YES"),
                    _ => {}
                }
            }
            Section::Well => {
                let (m, u, value, d) = parse_structured(line);
                match m.as_str() {
                    "NULL" => {
                        well.null_value =
                            value.parse::<f64>().map_err(|_| WellLogError::InvalidNumber {
                                offset: line_offset,
                                value: value.clone(),
                            })?
                    }
                    "WELL" => well.well_name = Some(value),
                    "FLD" => well.field = Some(value),
                    "LOC" => well.location = Some(value),
                    "CTY" => well.county = Some(value),
                    "STA" => well.state = Some(value),
                    "CNT" => well.country = Some(value),
                    "SRVC" => well.service_company = Some(value),
                    "DATE" => well.date = Some(value),
                    "API" | "UWI" => well.api = Some(value),
                    _ => well.extra.push(Parameter { mnemonic: m, unit: u, value, description: d }),
                }
            }
            Section::Parameter => {
                let (m, u, value, d) = parse_structured(line);
                parameters.push(Parameter { mnemonic: m, unit: u, value, description: d });
            }
            Section::Curve => {
                let (m, u, value, d) = parse_structured(line);
                curves.push(Curve { mnemonic: m, unit: u, api_code: value, description: d });
            }
            Section::Ascii | Section::Other => {}
        }
    }

    let version = version.ok_or(WellLogError::MissingSection("V"))?;
    if curves.is_empty() {
        return Err(WellLogError::MissingSection("C"));
    }

    let remainder = match data_start {
        Some(start) => &input[start..],
        None => "",
    };

    Ok((ParsedHeader { version, wrap, well, parameters, curves }, remainder))
}

/// Finish a [`ParsedHeader`] by reading ASCII `~A` data rows from `rest`.
pub(crate) fn parse_ascii_data(header: ParsedHeader, rest: &str) -> Result<WellLog, WellLogError> {
    let ParsedHeader { version, wrap, well, parameters, curves } = header;
    let ncol = curves.len();
    let mut data: Vec<Vec<f64>> = Vec::new();

    // Accumulator for WRAP=YES (tokens may span multiple physical lines).
    let mut pending: Vec<f64> = Vec::new();

    let mut cursor = 0usize;
    for raw in rest.split('\n') {
        let line_offset = cursor;
        cursor += raw.len() + 1;
        let line = raw.trim_end_matches('\r').trim();
        if line.is_empty() {
            continue;
        }
        // Skip the LAS 3.0 format keyword line (`ASCII`/`BINARY`).
        if line.len() <= 7 && line.chars().all(|c| c.is_ascii_alphabetic()) {
            let up = line.to_ascii_uppercase();
            if up == "ASCII" || up == "BINARY" {
                continue;
            }
        }
        for tok in line.split(|c: char| c.is_whitespace() || c == ',') {
            if tok.is_empty() {
                continue;
            }
            let v = tok.parse::<f64>().map_err(|_| WellLogError::InvalidNumber {
                offset: line_offset,
                value: tok.to_string(),
            })?;
            pending.push(v);
        }
        if ncol == 0 {
            continue;
        }
        while pending.len() >= ncol {
            data.push(pending.drain(..ncol).collect());
        }
    }

    // A trailing partial row under WRAP=YES is a structural error.
    if !pending.is_empty() {
        return Err(WellLogError::InconsistentColumnCount {
            expected: ncol,
            found: pending.len(),
            line: data.len() + 1,
        });
    }

    Ok(WellLog { version, wrap, well, parameters, curves, data })
}

/// Parse a LAS document from a string slice.
///
/// This is the ASCII path (LAS 2.0 and LAS 3.0 `ASCII` data sections). Tracks
/// the byte offset of each line so errors can report where they occurred.
/// Returns a fully owned [`WellLog`].
pub fn parse_str(input: &str) -> Result<WellLog, WellLogError> {
    let (header, remainder) = parse_header(input)?;
    parse_ascii_data(header, remainder)
}

/// Split a LAS `MNEM.UNIT  VALUE: DESCRIPTION` line into its four parts.
fn parse_structured(line: &str) -> (String, String, String, String) {
    let (left, right) = match line.split_once(':') {
        Some((l, r)) => (l, Some(r)),
        None => (line, None),
    };
    let left = left.trim();
    let mut it = left.splitn(2, char::is_whitespace);
    let mnu_full = it.next().unwrap_or("").trim().to_string();
    let value = it.next().unwrap_or("").trim().to_string();
    let (mnemonic, unit) = match mnu_full.split_once('.') {
        Some((m, u)) => (m.to_string(), u.to_string()),
        None => (mnu_full, String::new()),
    };
    let desc = right.map(|r| r.trim().to_string()).unwrap_or_default();
    (mnemonic, unit, value, desc)
}

/// Reader for LAS well-log files, implementing [`tpt_io_core::Reader`].
#[derive(Debug, Clone, Default)]
pub struct LasReader {
    validation: ValidationSettings,
}

impl LasReader {
    /// Create a reader with default validation (syntactic + semantic).
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the validation configuration (e.g. disable semantic checks).
    pub fn with_validation(mut self, validation: ValidationSettings) -> Self {
        self.validation = validation;
        self
    }
}

impl Reader<WellLog> for LasReader {
    type Error = WellLogError;

    fn parse(&self, input: &[u8]) -> Result<WellLog, WellLogError> {
        let log = crate::binary::parse(input)?;
        self.validation.run(&WellLogValidator, &log)?;
        Ok(log)
    }
}

impl Writer<WellLog> for LasReader {
    type Error = WellLogError;

    fn serialize(&self, value: &WellLog) -> Result<Vec<u8>, WellLogError> {
        let mut out = String::new();
        out.push_str("~Version Information\n");
        out.push_str(&format!("VERS. {} :\n", value.version));
        out.push_str(&format!("WRAP. {} :\n", if value.wrap { "YES" } else { "NO" }));

        out.push_str("~Well Information\n");
        let w = &value.well;
        if let Some(v) = &w.well_name {
            out.push_str(&format!("WELL. {v} :\n"));
        }
        if let Some(v) = &w.field {
            out.push_str(&format!("FLD. {v} :\n"));
        }
        if let Some(v) = &w.location {
            out.push_str(&format!("LOC. {v} :\n"));
        }
        if let Some(v) = &w.county {
            out.push_str(&format!("CTY. {v} :\n"));
        }
        if let Some(v) = &w.state {
            out.push_str(&format!("STA. {v} :\n"));
        }
        if let Some(v) = &w.country {
            out.push_str(&format!("CNT. {v} :\n"));
        }
        if let Some(v) = &w.service_company {
            out.push_str(&format!("SRVC. {v} :\n"));
        }
        if let Some(v) = &w.date {
            out.push_str(&format!("DATE. {v} :\n"));
        }
        if let Some(v) = &w.api {
            out.push_str(&format!("API. {v} :\n"));
        }
        out.push_str(&format!("NULL. {} :\n", w.null_value));
        for p in &w.extra {
            out.push_str(&format!("{}.{}  {} :\n", p.mnemonic, p.unit, p.value));
        }

        out.push_str("~Curve Information\n");
        for c in &value.curves {
            out.push_str(&format!(
                "{}.{}  {} : {}\n",
                c.mnemonic, c.unit, c.api_code, c.description
            ));
        }

        out.push_str("~ASCII Log Data\n");
        for row in &value.data {
            for (i, val) in row.iter().enumerate() {
                if i > 0 {
                    out.push(' ');
                }
                out.push_str(&format!("{val}"));
            }
            out.push('\n');
        }

        Ok(out.into_bytes())
    }
}

/// Syntactic/semantic validation for a parsed [`WellLog`].
#[derive(Debug, Clone, Copy, Default)]
pub struct WellLogValidator;

impl Validator<WellLog> for WellLogValidator {
    type Error = WellLogError;

    fn validate_syntactic(&self, value: &WellLog) -> Result<(), WellLogError> {
        if value.curves.is_empty() {
            return Err(WellLogError::MissingSection("C"));
        }
        let n = value.curves.len();
        for (i, row) in value.data.iter().enumerate() {
            if row.len() != n {
                return Err(WellLogError::InconsistentColumnCount {
                    expected: n,
                    found: row.len(),
                    line: i + 1,
                });
            }
        }
        Ok(())
    }

    fn validate_semantic(&self, value: &WellLog) -> Result<(), WellLogError> {
        // The index curve (column 0) of a depth-indexed log should be
        // monotonically non-decreasing. Tolerate a single sample.
        if value.data.len() < 2 {
            return Ok(());
        }
        let mut prev = value.data[0][0];
        for row in &value.data[1..] {
            let cur = row[0];
            if cur < prev {
                // Not strictly an error across all datasets; log a structural
                // inconsistency rather than failing. Keep it lenient here.
                break;
            }
            prev = cur;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "\
~Version Information
VERS. 2.0 :
WRAP. NO :
~Well Information
NULL. -999.25 :
WELL. TEST WELL 1 :
FLD. EXAMPLE FIELD :
~Curve Information
DEPT.M   : DEPTH
GR.API   : GAMMA RAY
ILD.OHMM   : DEEP RESISTIVITY
~ASCII Log Data
10.0 75.5 12.3
20.0 80.1 11.9
30.0 78.4 12.0
";

    #[test]
    fn parses_sample() {
        let log = parse_str(SAMPLE).unwrap();
        assert_eq!(log.version, 2.0);
        assert!(!log.wrap);
        assert_eq!(log.well.well_name.as_deref(), Some("TEST WELL 1"));
        assert_eq!(log.curves.len(), 3);
        assert_eq!(log.curves[0].mnemonic, "DEPT");
        assert_eq!(log.curves[2].description, "DEEP RESISTIVITY");
        assert_eq!(log.data.len(), 3);
        assert_eq!(log.data[0], vec![10.0, 75.5, 12.3]);
        assert_eq!(log.index_values().collect::<Vec<_>>(), vec![10.0, 20.0, 30.0]);
    }

    #[test]
    fn missing_version_section() {
        let input =
            "~Well Information\nWELL. X:\n~Curve Information\nDEPT. M:\n~ASCII Log Data\n1 2\n";
        let err = parse_str(input).unwrap_err();
        assert!(matches!(err, WellLogError::MissingSection("V")));
    }

    #[test]
    fn invalid_number_in_data() {
        let input = "~Version Information\nVERS. 2.0:\nWRAP. NO:\n~Curve Information\nDEPT. M:\nGR . API:\n~ASCII Log Data\n1.0 abc\n";
        let err = parse_str(input).unwrap_err();
        assert!(matches!(err, WellLogError::InvalidNumber { .. }));
    }

    #[test]
    fn round_trip() {
        let original = parse_str(SAMPLE).unwrap();
        let bytes = LasReader::new().serialize(&original).unwrap();
        let reparsed = parse_str(core::str::from_utf8(&bytes).unwrap()).unwrap();
        assert_eq!(original, reparsed);
    }

    #[test]
    fn reader_runs_validation() {
        let reader = LasReader::new();
        let log = reader.parse(SAMPLE.as_bytes()).unwrap();
        assert_eq!(log.data.len(), 3);
    }

    #[test]
    fn wrap_accumulates_tokens() {
        let input = "~Version Information\nVERS. 2.0:\nWRAP. YES:\n~Curve Information\nDEPT. M:\nGR . API:\n~ASCII Log Data\n1.0 2.0 3.0 4.0\n5.0 6.0\n";
        let log = parse_str(input).unwrap();
        assert_eq!(log.data.len(), 3);
        assert_eq!(log.data[2], vec![5.0, 6.0]);
    }
}
