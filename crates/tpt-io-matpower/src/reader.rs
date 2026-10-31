//! Reader for the MATPOWER case script: `mpc.field = value;` statements inside a
//! `function mpc = name` file.

use alloc::string::{String, ToString};
use alloc::vec::Vec;

use tpt_io_core::ParseError;

use crate::error::MatpowerError;
use crate::model::{Branch, Bus, Case, Gen, Value};

/// Parse a MATPOWER case file.
pub fn parse(input: &[u8]) -> Result<Case, MatpowerError> {
    let src = core::str::from_utf8(input)
        .map_err(|e| ParseError::custom(e.valid_up_to(), "case file is not valid UTF-8"))?;
    let mut r = Reader { src: src.as_bytes(), text: src, pos: 0 };
    let mut name = None;
    let mut fields: Vec<(String, Value)> = Vec::new();

    loop {
        r.skip_blank();
        if r.pos >= r.src.len() {
            break;
        }
        if r.src[r.pos..].starts_with(b"function") {
            let line = r.take_line();
            if let Some(eq) = line.find('=') {
                let n = line[eq + 1..].trim().trim_end_matches("()");
                if !n.is_empty() {
                    name = Some(n.to_string());
                }
            }
            continue;
        }
        if r.src[r.pos..].starts_with(b"mpc.") {
            let save = r.pos;
            r.pos += 4;
            if let Some(key) = r.identifier() {
                r.skip_inline_space();
                if r.peek() == Some(b'=') {
                    r.pos += 1;
                    r.skip_inline_space();
                    if r.is_expression() {
                        // A MATLAB expression such as `struct(...)`: not a literal, so skip it.
                        r.skip_statement();
                        continue;
                    }
                    let value = r.value()?;
                    r.skip_inline_space();
                    if r.peek() == Some(b';') {
                        r.pos += 1;
                    }
                    fields.push((key, value));
                    continue;
                }
            }
            r.pos = save;
        }
        r.skip_statement();
    }
    build(name, fields)
}

struct Reader<'a> {
    src: &'a [u8],
    text: &'a str,
    pos: usize,
}

impl<'a> Reader<'a> {
    fn peek(&self) -> Option<u8> {
        self.src.get(self.pos).copied()
    }

    /// Skip whitespace and `%` comments, including newlines.
    fn skip_blank(&mut self) {
        loop {
            match self.peek() {
                Some(b' ' | b'\t' | b'\r' | b'\n') => self.pos += 1,
                Some(b'%') => self.skip_to_eol(),
                _ => return,
            }
        }
    }

    fn skip_inline_space(&mut self) {
        while matches!(self.peek(), Some(b' ' | b'\t')) {
            self.pos += 1;
        }
    }

    /// Advance past the next newline (or to the end of input).
    fn skip_to_eol(&mut self) {
        while let Some(c) = self.peek() {
            self.pos += 1;
            if c == b'\n' {
                break;
            }
        }
    }

    /// Return the rest of the current line (without its newline) and move past it.
    fn take_line(&mut self) -> &'a str {
        let start = self.pos;
        self.skip_to_eol();
        let end = if self.src.get(self.pos.wrapping_sub(1)) == Some(&b'\n') {
            self.pos - 1
        } else {
            self.pos
        };
        &self.text[start..end]
    }

    /// Skip an unknown statement up to and including the next `;` or newline.
    fn skip_statement(&mut self) {
        while let Some(c) = self.peek() {
            self.pos += 1;
            if c == b';' || c == b'\n' {
                break;
            }
        }
    }

    fn identifier(&mut self) -> Option<String> {
        let start = self.pos;
        while matches!(self.peek(), Some(c) if c.is_ascii_alphanumeric() || c == b'_') {
            self.pos += 1;
        }
        if self.pos == start {
            None
        } else {
            Some(self.text[start..self.pos].to_string())
        }
    }

    /// True when the right-hand side starts with a name that is not `Inf` or `NaN`,
    /// i.e. a function call or variable rather than a literal.
    fn is_expression(&self) -> bool {
        match self.peek() {
            Some(c) if c.is_ascii_alphabetic() => {
                let mut end = self.pos;
                while matches!(self.src.get(end), Some(c) if c.is_ascii_alphanumeric() || *c == b'_') {
                    end += 1;
                }
                let word = &self.text[self.pos..end];
                !(word.eq_ignore_ascii_case("inf") || word.eq_ignore_ascii_case("nan"))
            }
            _ => false,
        }
    }

    fn value(&mut self) -> Result<Value, MatpowerError> {
        self.skip_blank();
        match self.peek() {
            Some(b'[') => self.matrix(),
            Some(b'{') => self.cell(),
            Some(q @ (b'\'' | b'"')) => Ok(Value::Text(self.quoted(q)?)),
            Some(_) => Ok(Value::Scalar(self.number()?)),
            None => Err(ParseError::eof(self.pos).into()),
        }
    }

    /// A quoted string; a doubled quote character stands for one literal quote.
    fn quoted(&mut self, quote: u8) -> Result<String, MatpowerError> {
        let open = self.pos;
        self.pos += 1;
        let mut out = String::new();
        let mut seg = self.pos;
        loop {
            match self.peek() {
                None | Some(b'\n') => {
                    return Err(ParseError::custom(open, "unterminated string").into());
                }
                Some(c) if c == quote => {
                    out.push_str(&self.text[seg..self.pos]);
                    self.pos += 1;
                    if self.peek() == Some(quote) {
                        out.push(quote as char);
                        self.pos += 1;
                        seg = self.pos;
                    } else {
                        return Ok(out);
                    }
                }
                Some(_) => self.pos += 1,
            }
        }
    }

    fn token(&mut self) -> &'a str {
        let start = self.pos;
        while let Some(c) = self.peek() {
            if matches!(c, b' ' | b'\t' | b'\r' | b'\n' | b',' | b';' | b']' | b'}' | b'%') {
                break;
            }
            self.pos += 1;
        }
        &self.text[start..self.pos]
    }

    fn number(&mut self) -> Result<f64, MatpowerError> {
        let at = self.pos;
        let tok = self.token();
        tok.parse::<f64>()
            .map_err(|_| ParseError::custom(at, "expected a number").into())
    }

    fn matrix(&mut self) -> Result<Value, MatpowerError> {
        self.pos += 1;
        let mut rows: Vec<Vec<f64>> = Vec::new();
        let mut cur: Vec<f64> = Vec::new();
        loop {
            match self.peek() {
                None => return Err(ParseError::eof(self.pos).into()),
                Some(b']') => {
                    self.pos += 1;
                    if !cur.is_empty() {
                        rows.push(cur);
                    }
                    return Ok(Value::Matrix(rows));
                }
                Some(b';' | b'\n') => {
                    self.pos += 1;
                    if !cur.is_empty() {
                        rows.push(core::mem::take(&mut cur));
                    }
                }
                Some(b' ' | b'\t' | b'\r' | b',') => self.pos += 1,
                Some(b'%') => self.skip_to_eol(),
                Some(b'.') if self.src[self.pos..].starts_with(b"...") => {
                    // Continuation: the row goes on after the next line break.
                    self.skip_to_eol();
                }
                Some(_) => cur.push(self.number()?),
            }
        }
    }

    fn cell(&mut self) -> Result<Value, MatpowerError> {
        self.pos += 1;
        let mut items = Vec::new();
        loop {
            self.skip_blank();
            match self.peek() {
                None => return Err(ParseError::eof(self.pos).into()),
                Some(b'}') => {
                    self.pos += 1;
                    return Ok(Value::Cell(items));
                }
                Some(b',' | b';') => self.pos += 1,
                Some(q @ (b'\'' | b'"')) => items.push(self.quoted(q)?),
                Some(_) => {
                    let word = self.token();
                    if word.is_empty() {
                        return Err(ParseError::custom(self.pos, "unexpected character in cell").into());
                    }
                    items.push(word.to_string());
                }
            }
        }
    }
}

fn build(name: Option<String>, fields: Vec<(String, Value)>) -> Result<Case, MatpowerError> {
    let mut case = Case { name, ..Case::default() };
    for (key, value) in fields {
        match (key.as_str(), value) {
            ("version", Value::Text(t)) => case.version = Some(t),
            ("version", Value::Scalar(x)) => case.version = Some(x.to_string()),
            ("baseMVA", Value::Scalar(x)) => case.base_mva = x,
            ("bus", Value::Matrix(m)) => case.bus = m.iter().map(|r| bus_row(r)).collect::<Result<_, _>>()?,
            ("gen", Value::Matrix(m)) => case.gen = m.iter().map(|r| gen_row(r)).collect::<Result<_, _>>()?,
            ("branch", Value::Matrix(m)) => {
                case.branch = m.iter().map(|r| branch_row(r)).collect::<Result<_, _>>()?
            }
            (_, v) => case.extra.push((key, v)),
        }
    }
    Ok(case)
}

fn uint(x: f64, what: &str) -> Result<u32, MatpowerError> {
    if x.is_finite() && x >= 0.0 && x.fract() == 0.0 && x <= u32::MAX as f64 {
        Ok(x as u32)
    } else {
        Err(MatpowerError::Invalid(alloc::format!("{what} must be a non-negative integer")))
    }
}

fn need(row: &[f64], n: usize, what: &str) -> Result<(), MatpowerError> {
    if row.len() < n {
        Err(MatpowerError::Invalid(alloc::format!("{what} row has fewer than {n} columns")))
    } else {
        Ok(())
    }
}

fn bus_row(r: &[f64]) -> Result<Bus, MatpowerError> {
    need(r, 13, "bus")?;
    Ok(Bus {
        id: uint(r[0], "bus number")?,
        kind: uint(r[1], "bus type")?,
        pd: r[2],
        qd: r[3],
        gs: r[4],
        bs: r[5],
        area: uint(r[6], "area")?,
        vm: r[7],
        va: r[8],
        base_kv: r[9],
        zone: uint(r[10], "zone")?,
        vmax: r[11],
        vmin: r[12],
    })
}

fn gen_row(r: &[f64]) -> Result<Gen, MatpowerError> {
    need(r, 10, "gen")?;
    Ok(Gen {
        bus: uint(r[0], "generator bus")?,
        pg: r[1],
        qg: r[2],
        qmax: r[3],
        qmin: r[4],
        vg: r[5],
        mbase: r[6],
        status: uint(r[7], "generator status")?,
        pmax: r[8],
        pmin: r[9],
    })
}

fn branch_row(r: &[f64]) -> Result<Branch, MatpowerError> {
    need(r, 11, "branch")?;
    Ok(Branch {
        from: uint(r[0], "from bus")?,
        to: uint(r[1], "to bus")?,
        r: r[2],
        x: r[3],
        b: r[4],
        rate_a: r[5],
        rate_b: r[6],
        rate_c: r[7],
        ratio: r[8],
        angle: r[9],
        status: uint(r[10], "branch status")?,
        angmin: r.get(11).copied().unwrap_or(-360.0),
        angmax: r.get(12).copied().unwrap_or(360.0),
    })
}
