//! Fixed-column PDB records: parsing and writing.

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::fmt::Write as _;

use tpt_io_core::ParseError;

use crate::error::PdbError;

/// The `HEADER` record.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Header {
    /// Classification (columns 11–50).
    pub classification: String,
    /// Deposition date (columns 51–59), as written.
    pub deposition_date: String,
    /// The four-character PDB id (columns 63–66).
    pub id_code: String,
}

/// An `ATOM` or `HETATM` record.
#[derive(Debug, Clone, PartialEq)]
pub struct Atom {
    /// `true` for `HETATM`.
    pub hetero: bool,
    /// Atom serial number (columns 7–11).
    pub serial: u32,
    /// Atom name (columns 13–16), trimmed.
    pub name: String,
    /// Alternate location indicator (column 17), or a space.
    pub alt_loc: char,
    /// Residue name (columns 18–20), trimmed.
    pub res_name: String,
    /// Chain identifier (column 22).
    pub chain: char,
    /// Residue sequence number (columns 23–26).
    pub res_seq: i32,
    /// Insertion code (column 27), or a space.
    pub icode: char,
    /// Position `[x, y, z]` in Ångström (columns 31–54).
    pub position: [f64; 3],
    /// Occupancy (columns 55–60).
    pub occupancy: f64,
    /// Temperature factor (columns 61–66).
    pub b_factor: f64,
    /// Element symbol (columns 77–78), trimmed.
    pub element: String,
}

/// A parsed PDB file.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Pdb {
    /// The `HEADER` record, if present.
    pub header: Option<Header>,
    /// The `TITLE` text, with continuation lines joined by a space.
    pub title: String,
    /// Atom records in file order.
    pub atoms: Vec<Atom>,
    /// `CONECT` records: the atom serial and its bonded serials.
    pub conect: Vec<(u32, Vec<u32>)>,
}

/// The text of columns `from..=to` (1-based), trimmed. Columns past the end of the record are empty.
fn cols(line: &str, from: usize, to: usize) -> &str {
    let bytes = line.as_bytes();
    let start = (from - 1).min(bytes.len());
    let end = to.min(bytes.len());
    line.get(start..end).unwrap_or("").trim()
}

fn col_char(line: &str, c: usize) -> char {
    line.as_bytes().get(c - 1).map(|b| *b as char).unwrap_or(' ')
}

fn float(s: &str, offset: usize) -> Result<f64, PdbError> {
    if s.is_empty() {
        return Ok(0.0);
    }
    s.parse::<f64>().map_err(|_| ParseError::custom(offset, "expected a number").into())
}

fn int<T: core::str::FromStr>(s: &str, offset: usize) -> Result<T, PdbError> {
    if s.is_empty() {
        return Err(ParseError::custom(offset, "expected an integer").into());
    }
    s.parse::<T>().map_err(|_| ParseError::custom(offset, "expected an integer").into())
}

/// Parse a PDB file.
pub fn parse(input: &[u8]) -> Result<Pdb, PdbError> {
    let text = core::str::from_utf8(input)
        .map_err(|e| ParseError::custom(e.valid_up_to(), "file is not valid UTF-8"))?;
    let mut pdb = Pdb::default();
    let mut offset = 0usize;
    let mut title_parts: Vec<String> = Vec::new();
    for line in text.lines() {
        let at = offset;
        offset += line.len() + 1;
        let record = cols(line, 1, 6);
        match record {
            "HEADER" => {
                pdb.header = Some(Header {
                    classification: cols(line, 11, 50).to_string(),
                    deposition_date: cols(line, 51, 59).to_string(),
                    id_code: cols(line, 63, 66).to_string(),
                });
            }
            "TITLE" => title_parts.push(cols(line, 11, 80).to_string()),
            "ATOM" | "HETATM" => {
                pdb.atoms.push(Atom {
                    hetero: record == "HETATM",
                    serial: int(cols(line, 7, 11), at)?,
                    name: cols(line, 13, 16).to_string(),
                    alt_loc: col_char(line, 17),
                    res_name: cols(line, 18, 20).to_string(),
                    chain: col_char(line, 22),
                    res_seq: int(cols(line, 23, 26), at)?,
                    icode: col_char(line, 27),
                    position: [
                        float(cols(line, 31, 38), at)?,
                        float(cols(line, 39, 46), at)?,
                        float(cols(line, 47, 54), at)?,
                    ],
                    occupancy: float(cols(line, 55, 60), at)?,
                    b_factor: float(cols(line, 61, 66), at)?,
                    element: cols(line, 77, 78).to_string(),
                });
            }
            "CONECT" => {
                let serial: u32 = int(cols(line, 7, 11), at)?;
                let mut bonded = Vec::new();
                for (from, to) in [(12, 16), (17, 21), (22, 26), (27, 31)] {
                    let s = cols(line, from, to);
                    if !s.is_empty() {
                        bonded.push(int(s, at)?);
                    }
                }
                pdb.conect.push((serial, bonded));
            }
            _ => {}
        }
    }
    pdb.title = title_parts.join(" ");
    let mut serials: Vec<u32> = pdb.atoms.iter().map(|a| a.serial).collect();
    serials.sort_unstable();
    if serials.windows(2).any(|w| w[0] == w[1]) {
        return Err(PdbError::Invalid("duplicate atom serial number".to_string()));
    }
    Ok(pdb)
}

/// Serialise a PDB file in its fixed columns. Coordinates and factors are written with three
/// decimals, as the format specifies.
pub fn write(p: &Pdb) -> Vec<u8> {
    let mut out = String::new();
    if let Some(h) = &p.header {
        let _ = writeln!(
            out,
            "HEADER    {:<40}{:<9}   {:<4}",
            h.classification, h.deposition_date, h.id_code
        );
    }
    let mut rest = p.title.as_str();
    let mut continuation = 1;
    while !rest.is_empty() || continuation == 1 {
        let take = rest.char_indices().nth(70).map(|(i, _)| i).unwrap_or(rest.len());
        let (chunk, tail) = rest.split_at(take);
        let number = if continuation == 1 { String::new() } else { continuation.to_string() };
        let _ = writeln!(out, "TITLE {number:>2}  {chunk}");
        rest = tail;
        continuation += 1;
        if rest.is_empty() {
            break;
        }
    }
    for a in &p.atoms {
        let record = if a.hetero { "HETATM" } else { "ATOM  " };
        let _ = writeln!(
            out,
            "{record}{:>5} {:<4}{}{:<3} {}{:>4}{}   {:>8.3}{:>8.3}{:>8.3}{:>6.2}{:>6.2}          {:>2}",
            a.serial,
            a.name,
            a.alt_loc,
            a.res_name,
            a.chain,
            a.res_seq,
            a.icode,
            a.position[0],
            a.position[1],
            a.position[2],
            a.occupancy,
            a.b_factor,
            a.element
        );
    }
    for (serial, bonded) in &p.conect {
        let mut line = alloc::format!("CONECT{serial:>5}");
        for b in bonded {
            let _ = write!(line, "{b:>5}");
        }
        out.push_str(&line);
        out.push('\n');
    }
    out.push_str("END\n");
    out.into_bytes()
}
