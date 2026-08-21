//! SPICE netlist parser (HSPICE / LTSpice / Ngspice / PSpice dialects).
//!
//! Parses element cards (`R`/`L`/`C`/`V`/`I`/`D`/`Q`/`M`/`E`/`F`/`G`/`H`/`T`/`X`/`B`),
//! `.model` definitions, `.subckt`/`.ends` blocks, and analysis commands
//! (`.tran`/`.ac`/`.op`/`.dc`/`.noise`). Component values with SI suffixes
//! (`k`, `u`, `n`, `p`, `meg`, …) are scaled to their numeric form. Parsing is
//! allocation-based and operates over a `&[u8]` input after UTF-8 detection.

use alloc::string::String;
use alloc::string::ToString;
use alloc::vec::Vec;

use tpt_io_core::{Encoding, Reader, ValidationSettings, Validator};

use crate::error::SpiceError;
use crate::model::{Analysis, Element, ElementKind, Model, SpiceNetlist, Subckt, Value};

/// Scale factor for an SI-ish suffix used by SPICE value syntax.
fn scale_of(suffix: &str) -> Option<f64> {
    match suffix.to_ascii_uppercase().as_str() {
        "" => Some(1.0),
        "F" => Some(1e-15),
        "P" => Some(1e-12),
        "N" => Some(1e-9),
        "U" | "MIC" => Some(1e-6),
        "MIL" => Some(25.4e-6),
        "M" => Some(1e-3),
        "K" => Some(1e3),
        "MEG" => Some(1e6),
        "G" => Some(1e9),
        "T" => Some(1e12),
        _ => None,
    }
}

/// Split a value token into its numeric prefix and trailing suffix.
fn split_num_suffix(tok: &str) -> (&str, &str) {
    let bytes = tok.as_bytes();
    let len = bytes.len();
    let mut i = 0;
    if i < len && (bytes[i] == b'+' || bytes[i] == b'-') {
        i += 1;
    }
    while i < len && bytes[i].is_ascii_digit() {
        i += 1;
    }
    if i < len && bytes[i] == b'.' {
        i += 1;
        while i < len && bytes[i].is_ascii_digit() {
            i += 1;
        }
    }
    if i < len && (bytes[i] == b'e' || bytes[i] == b'E') {
        i += 1;
        if i < len && (bytes[i] == b'+' || bytes[i] == b'-') {
            i += 1;
        }
        while i < len && bytes[i].is_ascii_digit() {
            i += 1;
        }
    }
    (&tok[..i], &tok[i..])
}

/// Parse a value token into a [`Value`], scaling any SI suffix.
pub fn parse_value(tok: &str) -> Value {
    let (num, suffix) = split_num_suffix(tok);
    let scalar = scale_of(suffix).and_then(|s| num.parse::<f64>().ok().map(|v| v * s));
    Value { raw: tok.to_string(), scalar }
}

fn parse_model(tokens: &[&str]) -> Result<Model, SpiceError> {
    if tokens.len() < 3 {
        return Err(SpiceError::MissingField("model name/type"));
    }
    let name = tokens[1].to_string();
    let device = tokens[2].to_string();
    let mut params: Vec<(String, String)> = Vec::new();
    for t in &tokens[3..] {
        let t = t.trim_matches(|c| c == '(' || c == ')' || c == ',');
        if t.is_empty() {
            continue;
        }
        if let Some((k, v)) = t.split_once('=') {
            params.push((k.trim().to_string(), v.trim().to_string()));
        } else {
            params.push((t.to_string(), String::new()));
        }
    }
    Ok(Model { name, device, params })
}

fn parse_subckt(tokens: &[&str]) -> Subckt {
    if tokens.len() < 2 {
        return Subckt::default();
    }
    let name = tokens[1].to_string();
    // `.subckt name (p1,p2) port1 port2 ...` — a parenthesised group holds the
    // formal parameters; everything after it (or after the name, if no group)
    // are the ports. A group may be a single token `(p1,p2)` or span multiple
    // tokens `(p1` … `p2)`.
    let mut params: Vec<String> = Vec::new();
    let mut ports: Vec<String> = Vec::new();
    let mut in_params = false;
    for raw in &tokens[2..] {
        let opens = raw.contains('(');
        let closes = raw.contains(')');
        let t = raw.trim_matches(|c| c == '(' || c == ')' || c == ',');
        if opens && closes {
            // Self-contained group like `(p1,p2)`.
            for p in t.split(',') {
                if !p.is_empty() {
                    params.push(p.to_string());
                }
            }
            continue;
        }
        if opens {
            in_params = true;
            if !t.is_empty() {
                params.push(t.to_string());
            }
            continue;
        }
        if closes {
            if !t.is_empty() {
                params.push(t.to_string());
            }
            in_params = false;
            continue;
        }
        if t.is_empty() {
            in_params = !in_params;
            continue;
        }
        if in_params {
            params.push(t.to_string());
        } else {
            ports.push(t.to_string());
        }
    }
    Subckt { name, params, ports, body: Vec::new() }
}

pub fn parse_element(tokens: &[&str]) -> Result<Element, SpiceError> {
    let first = tokens[0];
    let c = first.chars().next().ok_or(SpiceError::MissingField("element name"))?;
    let kind = ElementKind::from_designator(c).ok_or(SpiceError::UnknownElement(c))?;
    let rest = &tokens[1..];

    let node_count = match kind {
        ElementKind::R | ElementKind::L | ElementKind::C => 2,
        ElementKind::V | ElementKind::I | ElementKind::D => 2,
        ElementKind::Q => 3,
        ElementKind::M => 4,
        ElementKind::E | ElementKind::G => 4,
        ElementKind::F | ElementKind::H | ElementKind::B => 2,
        ElementKind::T => 4,
        ElementKind::O => 0,
        ElementKind::X => 0, // handled below
    };

    let (nodes, subckt, value, params): (Vec<String>, Option<String>, Option<Value>, Vec<String>) =
        if kind == ElementKind::X {
            if rest.is_empty() {
                return Err(SpiceError::MissingField("subckt instance"));
            }
            // `X1 n1 n2 ... subckt [params: k=v ...]` — the subcircuit name is the
            // last token that does not carry a `key=value` assignment; everything
            // after it (minus an optional `params:` keyword) are instance params.
            let eq = rest.iter().position(|t| t.contains('='));
            let (prefix, raw_params) = match eq {
                Some(i) => (&rest[..i], &rest[i..]),
                None => (rest, &[][..]),
            };
            let prefix: Vec<&str> = prefix
                .iter()
                .filter(|t| !t.eq_ignore_ascii_case("params:"))
                .copied()
                .collect();
            let subckt_name = prefix
                .last()
                .ok_or(SpiceError::MissingField("subckt name"))?
                .to_string();
            let nodes = prefix[..prefix.len() - 1].iter().map(|s| s.to_string()).collect();
            let params = raw_params.iter().map(|s| s.to_string()).collect();
            (nodes, Some(subckt_name), None, params)
        } else {
            if rest.len() < node_count {
                return Err(SpiceError::MissingField("nodes"));
            }
            let (nodes, tail) = rest.split_at(node_count);
            let (value, params): (Option<Value>, Vec<String>) = if matches!(
                kind,
                ElementKind::R | ElementKind::L | ElementKind::C
            ) {
                if tail.is_empty() {
                    (None, Vec::new())
                } else {
                    (
                        Some(parse_value(tail[0])),
                        tail[1..].iter().map(|s| s.to_string()).collect(),
                    )
                }
            } else {
                (None, tail.iter().map(|s| s.to_string()).collect())
            };
            (nodes.iter().map(|s| s.to_string()).collect(), None, value, params)
        };

    Ok(Element {
        kind,
        name: first.to_string(),
        nodes,
        value,
        params,
        subckt,
    })
}

/// Parse a SPICE netlist from a string slice.
pub fn parse_str(input: &str) -> Result<SpiceNetlist, SpiceError> {
    let mut net = SpiceNetlist::default();
    let mut pending = String::new();
    let mut title_set = false;
    let mut in_subckt: Option<usize> = None;

    for raw in input.lines() {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            continue;
        }
        // Strip inline `$` comments.
        let line = match trimmed.split_once('$') {
            Some((l, _)) => l.trim(),
            None => trimmed,
        };
        if line.is_empty() {
            continue;
        }

        // The first real (non-blank) line is the title, even if it is a `*`
        // comment line — a common SPICE convention. A first line that already
        // looks like a statement (element card or `.` command) means the file
        // has no title, so it is parsed normally instead.
        if !title_set {
            title_set = true;
            let looks_like_stmt = line.starts_with('.')
                || line.chars().next().is_some_and(|c| ElementKind::from_designator(c).is_some());
            if !looks_like_stmt {
                net.title = Some(line.trim_start_matches('*').trim().to_string());
                continue;
            }
        }
        if line.starts_with('*') {
            continue;
        }

        if let Some(rest) = line.strip_prefix('+') {
            pending.push(' ');
            pending.push_str(rest.trim());
            continue;
        }
        if !pending.is_empty() {
            let done = core::mem::take(&mut pending);
            handle(&done, &mut net, &mut in_subckt);
        }
        pending = line.to_string();
    }
    if !pending.is_empty() {
        handle(&pending, &mut net, &mut in_subckt);
    }

    Ok(net)
}

fn handle(stmt: &str, net: &mut SpiceNetlist, in_subckt: &mut Option<usize>) {
    let tokens: Vec<&str> = stmt.split_whitespace().collect();
    if tokens.is_empty() {
        return;
    }

    if let Some(idx) = *in_subckt {
        if tokens[0].eq_ignore_ascii_case(".ends") {
            *in_subckt = None;
            return;
        }
        net.subckts[idx].body.push(stmt.to_string());
        return;
    }

    if tokens[0].starts_with('.') {
        match tokens[0].to_ascii_lowercase().as_str() {
            ".model" => {
                if let Ok(m) = parse_model(&tokens) {
                    net.models.push(m);
                }
            }
            ".subckt" => {
                let sub = parse_subckt(&tokens);
                net.subckts.push(sub);
                *in_subckt = Some(net.subckts.len() - 1);
            }
            ".ends" => *in_subckt = None,
            ".tran" => net.analyses.push(Analysis::Tran(args(&tokens))),
            ".ac" => net.analyses.push(Analysis::Ac(args(&tokens))),
            ".op" => net.analyses.push(Analysis::Op),
            ".dc" => net.analyses.push(Analysis::Dc(args(&tokens))),
            ".noise" => net.analyses.push(Analysis::Noise(args(&tokens))),
            ".temp" => {
                if let Some(t) = tokens.get(1).and_then(|s| s.parse::<f64>().ok()) {
                    net.temperature = Some(t);
                }
            }
            ".options" => {
                // `.options temp=25` / `.options TEMP=25`.
                if let Some(t) = tokens
                    .iter()
                    .find_map(|s| s.split_once('='))
                    .filter(|(k, _)| k.eq_ignore_ascii_case("temp"))
                    .and_then(|(_, v)| v.trim().parse::<f64>().ok())
                {
                    net.temperature = Some(t);
                }
            }
            other => {
                let name = other.trim_start_matches('.').to_string();
                if name == "end" {
                    return;
                }
                net.analyses.push(Analysis::Other(name, args(&tokens)));
            }
        }
        return;
    }

    if let Some(c) = tokens[0].chars().next() {
        if ElementKind::from_designator(c).is_some() {
            if let Ok(el) = parse_element(&tokens) {
                net.elements.push(el);
            }
        }
    }
}

fn args(tokens: &[&str]) -> Vec<String> {
    tokens[1..].iter().map(|s| s.to_string()).collect()
}

/// Reader for SPICE netlists, implementing [`tpt_io_core::Reader`].
#[derive(Debug, Clone, Default)]
pub struct SpiceReader {
    validation: ValidationSettings,
}

impl SpiceReader {
    /// Create a reader with default validation (syntactic + semantic).
    pub fn new() -> Self {
        Self {
            validation: ValidationSettings::new(),
        }
    }

    /// Set the validation configuration.
    pub fn with_validation(mut self, validation: ValidationSettings) -> Self {
        self.validation = validation;
        self
    }
}

impl Reader<SpiceNetlist> for SpiceReader {
    type Error = SpiceError;

    fn parse(&self, input: &[u8]) -> Result<SpiceNetlist, SpiceError> {
        let text = Encoding::detect(input)
            .as_str(input)
            .ok_or(SpiceError::Unsupported("non-UTF-8 SPICE input"))?;
        let net = parse_str(text)?;
        self.validation.run(&SpiceValidator, &net)?;
        Ok(net)
    }
}

impl tpt_io_core::Writer<SpiceNetlist> for SpiceReader {
    type Error = SpiceError;

    fn serialize(&self, value: &SpiceNetlist) -> Result<Vec<u8>, SpiceError> {
        crate::writer::serialize(value)
    }
}

/// Syntactic / semantic validation for a parsed [`SpiceNetlist`].
#[derive(Debug, Clone, Copy, Default)]
pub struct SpiceValidator;

impl Validator<SpiceNetlist> for SpiceValidator {
    type Error = SpiceError;

    fn validate_syntactic(&self, value: &SpiceNetlist) -> Result<(), SpiceError> {
        for el in &value.elements {
            if matches!(el.kind, ElementKind::R | ElementKind::L | ElementKind::C)
                && el.value.is_none()
            {
                return Err(SpiceError::MissingField("value"));
            }
        }
        for m in &value.models {
            if m.device.is_empty() {
                return Err(SpiceError::MissingField("model device"));
            }
        }
        Ok(())
    }

    fn validate_semantic(&self, _value: &SpiceNetlist) -> Result<(), SpiceError> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NETLIST: &str = "\
* RC low-pass filter
R1 in out 1k
C1 out 0 100n
V1 in 0 DC 5 AC 1
M1 d g s b nmos W=2u L=0.5u
.model nmos nmos (VTO=0.7 KP=120u)
.tran 1u 10m
.ac dec 10 1 1MEG
.subckt inv a y vdd vss
R1 a y 1k
.ends
";

    #[test]
    fn parses_elements() {
        let net = parse_str(NETLIST).unwrap();
        assert_eq!(net.title.as_deref(), Some("RC low-pass filter"));
        // Title should be the descriptive first line, not the comment.
        let r = net.elements.iter().find(|e| e.name == "R1").unwrap();
        assert_eq!(r.kind, ElementKind::R);
        assert_eq!(r.nodes, vec!["in", "out"]);
        assert_eq!(r.value.as_ref().unwrap().scalar, Some(1000.0));

        let c = net.elements.iter().find(|e| e.name == "C1").unwrap();
        let c_val = c.value.as_ref().unwrap().scalar.unwrap();
        assert!((c_val - 100e-9).abs() < 1e-12, "C1 value was {c_val}");

        let m = net.elements.iter().find(|e| e.name == "M1").unwrap();
        assert_eq!(m.kind, ElementKind::M);
        assert_eq!(m.nodes, vec!["d", "g", "s", "b"]);
        assert_eq!(m.params.first().unwrap(), "nmos");
    }

    #[test]
    fn parses_model_and_analysis() {
        let net = parse_str(NETLIST).unwrap();
        let m = net.models.iter().find(|m| m.name == "nmos").unwrap();
        assert_eq!(m.device, "nmos");
        assert!(m.params.iter().any(|(k, _)| k == "VTO"));

        assert!(net.analyses.iter().any(|a| matches!(a, Analysis::Tran(_))));
        assert!(net.analyses.iter().any(|a| matches!(a, Analysis::Ac(_))));
    }

    #[test]
    fn parses_subckt() {
        let net = parse_str(NETLIST).unwrap();
        let sub = net.subckts.iter().find(|s| s.name == "inv").unwrap();
        assert_eq!(sub.ports, vec!["a", "y", "vdd", "vss"]);
        assert_eq!(sub.body.len(), 1);
    }

    #[test]
    fn value_suffixes() {
        assert_eq!(parse_value("1k").scalar, Some(1000.0));
        assert_eq!(parse_value("2.2u").scalar, Some(2.2e-6));
        assert_eq!(parse_value("1MEG").scalar, Some(1e6));
        assert_eq!(parse_value("1e3").scalar, Some(1000.0));
        assert_eq!(parse_value("R=1k").scalar, None); // not a plain number
    }

    #[test]
    fn reader_runs_validation() {
        use tpt_io_core::Reader;
        let net = SpiceReader::new().parse(NETLIST.as_bytes()).unwrap();
        assert!(net.elements.len() >= 4);
    }

    #[test]
    fn continuation_lines() {
        let src = "R1 a b 1k\n+ TC=0.001\nC1 b 0 10p";
        let net = parse_str(src).unwrap();
        let r = net.elements.iter().find(|e| e.name == "R1").unwrap();
        assert!(r.params.iter().any(|p| p.starts_with("TC=")));
    }
}
