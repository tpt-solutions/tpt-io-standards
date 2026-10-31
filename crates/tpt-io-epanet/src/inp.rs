//! `.inp` parsing, writing and typed network views.

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::fmt::Write as _;

use tpt_io_core::ParseError;

use crate::error::EpanetError;

/// One row of a section: its whitespace-separated tokens and any trailing or whole-line comment.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Row {
    /// The tokens, in order. Empty for a comment-only row.
    pub tokens: Vec<String>,
    /// The text after `;`, trimmed, if present.
    pub comment: Option<String>,
}

/// A `[NAME]` section and its rows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Section {
    /// The section name without brackets, e.g. `JUNCTIONS`.
    pub name: String,
    /// The rows in file order.
    pub rows: Vec<Row>,
}

/// A parsed `.inp` file.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Inp {
    /// Comment and blank rows that appear before the first section.
    pub preamble: Vec<Row>,
    /// Sections in file order.
    pub sections: Vec<Section>,
}

/// A junction (`[JUNCTIONS]`).
#[derive(Debug, Clone, PartialEq)]
pub struct Junction {
    /// Node id.
    pub id: String,
    /// Elevation, in the network's length units.
    pub elevation: f64,
    /// Base demand, in the network's flow units.
    pub demand: f64,
    /// Demand pattern id, if given.
    pub pattern: Option<String>,
}

/// A reservoir (`[RESERVOIRS]`).
#[derive(Debug, Clone, PartialEq)]
pub struct Reservoir {
    /// Node id.
    pub id: String,
    /// Total head.
    pub head: f64,
    /// Head pattern id, if given.
    pub pattern: Option<String>,
}

/// A tank (`[TANKS]`).
#[derive(Debug, Clone, PartialEq)]
pub struct Tank {
    /// Node id.
    pub id: String,
    /// Bottom elevation.
    pub elevation: f64,
    /// Initial water level above the bottom.
    pub init_level: f64,
    /// Minimum water level.
    pub min_level: f64,
    /// Maximum water level.
    pub max_level: f64,
    /// Diameter (for cylindrical tanks).
    pub diameter: f64,
    /// Minimum volume.
    pub min_volume: f64,
}

/// A pipe (`[PIPES]`).
#[derive(Debug, Clone, PartialEq)]
pub struct Pipe {
    /// Link id.
    pub id: String,
    /// Start node id.
    pub node1: String,
    /// End node id.
    pub node2: String,
    /// Length.
    pub length: f64,
    /// Diameter.
    pub diameter: f64,
    /// Roughness coefficient.
    pub roughness: f64,
    /// Minor loss coefficient.
    pub minor_loss: f64,
    /// `Open`, `Closed` or `CV`, as written; `Open` when absent.
    pub status: String,
}

/// A node position (`[COORDINATES]`).
#[derive(Debug, Clone, PartialEq)]
pub struct Coordinate {
    /// Node id.
    pub id: String,
    /// X position.
    pub x: f64,
    /// Y position.
    pub y: f64,
}

/// Parse an `.inp` file.
pub fn parse(input: &[u8]) -> Result<Inp, EpanetError> {
    let text = core::str::from_utf8(input)
        .map_err(|e| ParseError::custom(e.valid_up_to(), "input is not valid UTF-8"))?;
    let mut inp = Inp::default();
    let mut offset = 0usize;
    for raw in text.split('\n') {
        let line_offset = offset;
        offset += raw.len() + 1;
        let line = raw.trim_end_matches('\r');
        let (code, comment) = match line.find(';') {
            Some(i) => (&line[..i], Some(line[i + 1..].trim().to_string())),
            None => (line, None),
        };
        let code = code.trim();
        if let Some(name) = code.strip_prefix('[') {
            let name = name
                .strip_suffix(']')
                .ok_or_else(|| ParseError::custom(line_offset, "section header is not closed"))?;
            inp.sections.push(Section { name: name.trim().to_ascii_uppercase(), rows: Vec::new() });
            continue;
        }
        let tokens: Vec<String> = code.split_whitespace().map(|t| t.to_string()).collect();
        if tokens.is_empty() && comment.is_none() {
            continue;
        }
        let row = Row { tokens, comment };
        match inp.sections.last_mut() {
            Some(s) => s.rows.push(row),
            None if row.tokens.is_empty() => inp.preamble.push(row),
            None => {
                return Err(ParseError::custom(line_offset, "row before the first section").into())
            }
        }
    }
    Ok(inp)
}

fn num(tok: &str, what: &str) -> Result<f64, EpanetError> {
    tok.parse::<f64>()
        .map_err(|_| EpanetError::Invalid(alloc::format!("{what} '{tok}' is not a number")))
}

fn need(row: &Row, n: usize, section: &str) -> Result<(), EpanetError> {
    if row.tokens.len() < n {
        Err(EpanetError::Invalid(alloc::format!("{section} row needs at least {n} fields")))
    } else {
        Ok(())
    }
}

impl Inp {
    /// The first section with the given name (case-insensitive).
    pub fn section(&self, name: &str) -> Option<&Section> {
        self.sections.iter().find(|s| s.name.eq_ignore_ascii_case(name))
    }

    /// Rows of the named section that have tokens (comment-only rows are skipped).
    fn data_rows(&self, name: &str) -> impl Iterator<Item = &Row> + '_ {
        self.section(name).into_iter().flat_map(|s| s.rows.iter()).filter(|r| !r.tokens.is_empty())
    }

    /// The `[JUNCTIONS]` rows.
    pub fn junctions(&self) -> Result<Vec<Junction>, EpanetError> {
        self.data_rows("JUNCTIONS")
            .map(|r| {
                need(r, 2, "JUNCTIONS")?;
                Ok(Junction {
                    id: r.tokens[0].clone(),
                    elevation: num(&r.tokens[1], "elevation")?,
                    demand: r.tokens.get(2).map(|t| num(t, "demand")).transpose()?.unwrap_or(0.0),
                    pattern: r.tokens.get(3).cloned(),
                })
            })
            .collect()
    }

    /// The `[RESERVOIRS]` rows.
    pub fn reservoirs(&self) -> Result<Vec<Reservoir>, EpanetError> {
        self.data_rows("RESERVOIRS")
            .map(|r| {
                need(r, 2, "RESERVOIRS")?;
                Ok(Reservoir {
                    id: r.tokens[0].clone(),
                    head: num(&r.tokens[1], "head")?,
                    pattern: r.tokens.get(2).cloned(),
                })
            })
            .collect()
    }

    /// The `[TANKS]` rows.
    pub fn tanks(&self) -> Result<Vec<Tank>, EpanetError> {
        self.data_rows("TANKS")
            .map(|r| {
                need(r, 6, "TANKS")?;
                let f = |i: usize, what: &str| num(&r.tokens[i], what);
                Ok(Tank {
                    id: r.tokens[0].clone(),
                    elevation: f(1, "elevation")?,
                    init_level: f(2, "initial level")?,
                    min_level: f(3, "minimum level")?,
                    max_level: f(4, "maximum level")?,
                    diameter: f(5, "diameter")?,
                    min_volume: r
                        .tokens
                        .get(6)
                        .map(|t| num(t, "minimum volume"))
                        .transpose()?
                        .unwrap_or(0.0),
                })
            })
            .collect()
    }

    /// The `[PIPES]` rows.
    pub fn pipes(&self) -> Result<Vec<Pipe>, EpanetError> {
        self.data_rows("PIPES")
            .map(|r| {
                need(r, 6, "PIPES")?;
                let f = |i: usize, what: &str| num(&r.tokens[i], what);
                Ok(Pipe {
                    id: r.tokens[0].clone(),
                    node1: r.tokens[1].clone(),
                    node2: r.tokens[2].clone(),
                    length: f(3, "length")?,
                    diameter: f(4, "diameter")?,
                    roughness: f(5, "roughness")?,
                    minor_loss: r
                        .tokens
                        .get(6)
                        .map(|t| num(t, "minor loss"))
                        .transpose()?
                        .unwrap_or(0.0),
                    status: r.tokens.get(7).cloned().unwrap_or_else(|| "Open".to_string()),
                })
            })
            .collect()
    }

    /// The `[COORDINATES]` rows.
    pub fn coordinates(&self) -> Result<Vec<Coordinate>, EpanetError> {
        self.data_rows("COORDINATES")
            .map(|r| {
                need(r, 3, "COORDINATES")?;
                Ok(Coordinate {
                    id: r.tokens[0].clone(),
                    x: num(&r.tokens[1], "x")?,
                    y: num(&r.tokens[2], "y")?,
                })
            })
            .collect()
    }

    /// Check the network: node ids are unique across junctions, reservoirs and tanks; every pipe
    /// joins two known nodes and has positive length and diameter; each pipe status is `Open`,
    /// `Closed` or `CV`.
    pub fn validate(&self) -> Result<(), EpanetError> {
        let mut nodes: Vec<String> = Vec::new();
        nodes.extend(self.junctions()?.into_iter().map(|j| j.id));
        nodes.extend(self.reservoirs()?.into_iter().map(|r| r.id));
        nodes.extend(self.tanks()?.into_iter().map(|t| t.id));
        let mut sorted = nodes.clone();
        sorted.sort_unstable();
        if sorted.windows(2).any(|w| w[0] == w[1]) {
            return Err(EpanetError::Topology("duplicate node id".to_string()));
        }
        for p in self.pipes()? {
            if !nodes.contains(&p.node1) || !nodes.contains(&p.node2) {
                return Err(EpanetError::Topology(alloc::format!(
                    "pipe {} joins an unknown node",
                    p.id
                )));
            }
            if !(p.length > 0.0 && p.diameter > 0.0) {
                return Err(EpanetError::Topology(alloc::format!(
                    "pipe {} needs positive length and diameter",
                    p.id
                )));
            }
            if !matches!(p.status.as_str(), "Open" | "Closed" | "CV") {
                return Err(EpanetError::Topology(alloc::format!(
                    "pipe {} has status '{}'",
                    p.id,
                    p.status
                )));
            }
        }
        Ok(())
    }
}

/// Serialise an `.inp` file. Rows are written as their tokens joined by tabs, with the comment
/// after `;`. `parse(write(x))` gives back the same rows.
pub fn write(inp: &Inp) -> Vec<u8> {
    let mut out = String::new();
    for r in &inp.preamble {
        write_row(&mut out, r);
    }
    for s in &inp.sections {
        let _ = writeln!(out, "[{}]", s.name);
        for r in &s.rows {
            write_row(&mut out, r);
        }
    }
    out.into_bytes()
}

fn write_row(out: &mut String, r: &Row) {
    let body = r.tokens.join("\t");
    match (&r.comment, body.is_empty()) {
        (Some(c), true) => {
            let _ = writeln!(out, ";{c}");
        }
        (Some(c), false) => {
            let _ = writeln!(out, "{body}\t;{c}");
        }
        (None, _) => {
            let _ = writeln!(out, "{body}");
        }
    }
}
