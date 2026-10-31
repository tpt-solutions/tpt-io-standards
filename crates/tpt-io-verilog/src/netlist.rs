//! Netlist model, tokenizer, parser and writer.

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::fmt::Write as _;

use tpt_io_core::ParseError;

use crate::error::VerilogError;

/// A port direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    /// `input`.
    Input,
    /// `output`.
    Output,
    /// `inout`.
    Inout,
}

/// A module port: its direction, optional bit range text, and name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Port {
    /// Direction.
    pub direction: Direction,
    /// The `[msb:lsb]` range as written, if any.
    pub range: Option<String>,
    /// Port name.
    pub name: String,
}

/// One connection of an instance: a port name (for named connections) and the net it joins.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Connection {
    /// The port name for `.PORT(net)`, or `None` for a positional connection.
    pub port: Option<String>,
    /// The net expression as written (`n1`, `bus[3]`, `1'b0`).
    pub net: String,
}

/// A cell or module instance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Instance {
    /// The cell or module type.
    pub cell: String,
    /// The instance name.
    pub name: String,
    /// Connections in file order.
    pub connections: Vec<Connection>,
}

/// A module.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Module {
    /// Module name.
    pub name: String,
    /// Names listed in the module header, in order.
    pub header_ports: Vec<String>,
    /// Port declarations with directions.
    pub ports: Vec<Port>,
    /// Internal `wire` declarations (name, optional range).
    pub wires: Vec<(Option<String>, String)>,
    /// Instances in file order.
    pub instances: Vec<Instance>,
    /// `assign lhs = rhs;` statements.
    pub assigns: Vec<(String, String)>,
}

/// A netlist: its modules in file order.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Netlist {
    /// Modules.
    pub modules: Vec<Module>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Tok {
    Ident(String),
    Punct(char),
}

/// Parse a structural Verilog netlist.
pub fn parse(input: &[u8]) -> Result<Netlist, VerilogError> {
    let src = core::str::from_utf8(input)
        .map_err(|e| ParseError::custom(e.valid_up_to(), "netlist is not valid UTF-8"))?;
    let toks = tokenize(src)?;
    let mut p = Parser { toks, pos: 0 };
    let mut netlist = Netlist::default();
    while p.pos < p.toks.len() {
        if p.keyword("module") {
            netlist.modules.push(p.module()?);
        } else {
            return Err(p.error("expected 'module'"));
        }
    }
    Ok(netlist)
}

fn tokenize(src: &str) -> Result<Vec<(usize, Tok)>, VerilogError> {
    let bytes = src.as_bytes();
    let mut out = Vec::new();
    let mut i = 0usize;
    while i < bytes.len() {
        let c = bytes[i];
        if c.is_ascii_whitespace() {
            i += 1;
        } else if src[i..].starts_with("//") {
            while i < bytes.len() && bytes[i] != b'\n' {
                i += 1;
            }
        } else if src[i..].starts_with("/*") {
            match src[i + 2..].find("*/") {
                Some(end) => i += 2 + end + 2,
                None => return Err(ParseError::eof(bytes.len()).into()),
            }
        } else if c.is_ascii_alphabetic() || c == b'_' || c == b'\\' {
            let start = i;
            if c == b'\\' {
                // Escaped identifier: runs to the next whitespace.
                while i < bytes.len() && !bytes[i].is_ascii_whitespace() {
                    i += 1;
                }
            } else {
                while i < bytes.len()
                    && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_' || bytes[i] == b'$')
                {
                    i += 1;
                }
            }
            out.push((start, Tok::Ident(src[start..i].to_string())));
        } else if c.is_ascii_digit() {
            // Numbers and sized literals such as 1'b0 are kept as identifiers for connection purposes.
            let start = i;
            while i < bytes.len()
                && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'\'' || bytes[i] == b'_')
            {
                i += 1;
            }
            out.push((start, Tok::Ident(src[start..i].to_string())));
        } else if b"()[]{};,.:=#@".contains(&c) {
            out.push((i, Tok::Punct(c as char)));
            i += 1;
        } else {
            return Err(ParseError::invalid_byte(i, c).into());
        }
    }
    Ok(out)
}

struct Parser {
    toks: Vec<(usize, Tok)>,
    pos: usize,
}

impl Parser {
    fn error(&self, what: &str) -> VerilogError {
        let at = self.toks.get(self.pos).map(|(o, _)| *o).unwrap_or(0);
        ParseError::custom(at, what).into()
    }

    fn peek_ident(&self) -> Option<&str> {
        match self.toks.get(self.pos) {
            Some((_, Tok::Ident(s))) => Some(s),
            _ => None,
        }
    }

    fn peek_punct(&self, c: char) -> bool {
        matches!(self.toks.get(self.pos), Some((_, Tok::Punct(p))) if *p == c)
    }

    fn keyword(&mut self, kw: &str) -> bool {
        if self.peek_ident() == Some(kw) {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    fn ident(&mut self) -> Result<String, VerilogError> {
        match self.toks.get(self.pos) {
            Some((_, Tok::Ident(s))) => {
                let s = s.clone();
                self.pos += 1;
                Ok(s)
            }
            _ => Err(self.error("expected an identifier")),
        }
    }

    /// A net expression: a name, optionally with a bit or part select (`bus[3]`, `bus[7:0]`).
    fn net_expr(&mut self) -> Result<String, VerilogError> {
        let mut text = self.ident()?;
        if self.peek_punct('[') {
            if let Some(sel) = self.range()? {
                text.push_str(&sel);
            }
        }
        Ok(text)
    }

    fn punct(&mut self, c: char) -> Result<(), VerilogError> {
        if self.peek_punct(c) {
            self.pos += 1;
            Ok(())
        } else {
            Err(self.error(&alloc::format!("expected '{c}'")))
        }
    }

    /// A `[msb:lsb]` range, kept as text (without the brackets' whitespace).
    fn range(&mut self) -> Result<Option<String>, VerilogError> {
        if !self.peek_punct('[') {
            return Ok(None);
        }
        self.pos += 1;
        let mut text = String::new();
        loop {
            match self.toks.get(self.pos) {
                Some((_, Tok::Punct(']'))) => {
                    self.pos += 1;
                    return Ok(Some(alloc::format!("[{text}]")));
                }
                Some((_, Tok::Ident(s))) => {
                    text.push_str(s);
                    self.pos += 1;
                }
                Some((_, Tok::Punct(c))) => {
                    text.push(*c);
                    self.pos += 1;
                }
                None => return Err(self.error("unterminated range")),
            }
        }
    }

    fn module(&mut self) -> Result<Module, VerilogError> {
        let mut m = Module { name: self.ident()?, ..Module::default() };
        if self.peek_punct('#') {
            return Err(VerilogError::Unsupported("module parameters".to_string()));
        }
        if self.peek_punct('(') {
            self.pos += 1;
            while !self.peek_punct(')') {
                if self.peek_ident().is_some_and(|k| matches!(k, "input" | "output" | "inout")) {
                    // ANSI-style port declaration in the header.
                    let dir = direction(&self.ident()?);
                    let _ = self.keyword("wire");
                    let range = self.range()?;
                    let name = self.ident()?;
                    m.header_ports.push(name.clone());
                    m.ports.push(Port { direction: dir, range, name });
                } else {
                    m.header_ports.push(self.ident()?);
                }
                if self.peek_punct(',') {
                    self.pos += 1;
                }
            }
            self.pos += 1;
        }
        self.punct(';')?;
        loop {
            if self.keyword("endmodule") {
                return Ok(m);
            }
            match self.peek_ident() {
                Some(k @ ("input" | "output" | "inout")) => {
                    let dir = direction(k);
                    self.pos += 1;
                    let _ = self.keyword("wire");
                    let range = self.range()?;
                    loop {
                        let name = self.ident()?;
                        m.ports.push(Port { direction: dir, range: range.clone(), name });
                        if self.peek_punct(',') {
                            self.pos += 1;
                        } else {
                            break;
                        }
                    }
                    self.punct(';')?;
                }
                Some("wire") => {
                    self.pos += 1;
                    let range = self.range()?;
                    loop {
                        let name = self.ident()?;
                        m.wires.push((range.clone(), name));
                        if self.peek_punct(',') {
                            self.pos += 1;
                        } else {
                            break;
                        }
                    }
                    self.punct(';')?;
                }
                Some("assign") => {
                    self.pos += 1;
                    let lhs = self.net_expr()?;
                    self.punct('=')?;
                    let rhs = self.net_expr()?;
                    self.punct(';')?;
                    m.assigns.push((lhs, rhs));
                }
                Some(
                    kw @ ("always" | "initial" | "function" | "task" | "reg" | "parameter"
                    | "generate"),
                ) => {
                    return Err(VerilogError::Unsupported(kw.to_string()));
                }
                Some(_) => m.instances.push(self.instance()?),
                None => return Err(self.error("expected a declaration, instance or endmodule")),
            }
        }
    }

    fn instance(&mut self) -> Result<Instance, VerilogError> {
        let cell = self.ident()?;
        let name = self.ident()?;
        self.punct('(')?;
        let mut connections = Vec::new();
        while !self.peek_punct(')') {
            if self.peek_punct('.') {
                self.pos += 1;
                let port = self.ident()?;
                self.punct('(')?;
                let net = if self.peek_punct(')') { String::new() } else { self.net_expr()? };
                self.punct(')')?;
                connections.push(Connection { port: Some(port), net });
            } else {
                connections.push(Connection { port: None, net: self.net_expr()? });
            }
            if self.peek_punct(',') {
                self.pos += 1;
            }
        }
        self.pos += 1;
        self.punct(';')?;
        Ok(Instance { cell, name, connections })
    }
}

fn direction(k: &str) -> Direction {
    match k {
        "input" => Direction::Input,
        "output" => Direction::Output,
        _ => Direction::Inout,
    }
}

/// Serialise a netlist in the same structural subset.
pub fn write(n: &Netlist) -> Vec<u8> {
    let mut out = String::new();
    for m in &n.modules {
        let _ = writeln!(out, "module {} ({});", m.name, m.header_ports.join(", "));
        for p in &m.ports {
            let d = match p.direction {
                Direction::Input => "input",
                Direction::Output => "output",
                Direction::Inout => "inout",
            };
            match &p.range {
                Some(r) => {
                    let _ = writeln!(out, "  {d} {r} {};", p.name);
                }
                None => {
                    let _ = writeln!(out, "  {d} {};", p.name);
                }
            }
        }
        for (range, name) in &m.wires {
            match range {
                Some(r) => {
                    let _ = writeln!(out, "  wire {r} {name};");
                }
                None => {
                    let _ = writeln!(out, "  wire {name};");
                }
            }
        }
        for (lhs, rhs) in &m.assigns {
            let _ = writeln!(out, "  assign {lhs} = {rhs};");
        }
        for i in &m.instances {
            let conns: Vec<String> = i
                .connections
                .iter()
                .map(|c| match &c.port {
                    Some(p) => alloc::format!(".{p}({})", c.net),
                    None => c.net.clone(),
                })
                .collect();
            let _ = writeln!(out, "  {} {} ({});", i.cell, i.name, conns.join(", "));
        }
        out.push_str("endmodule\n");
    }
    out.into_bytes()
}
