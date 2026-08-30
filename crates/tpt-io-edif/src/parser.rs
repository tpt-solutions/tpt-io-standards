//! EDIF parser: token stream → S-expression tree → typed [`model::Edif`].
//!
//! The parser is intentionally two-stage. A small [`Sexp`] AST captures the full
//! document (so round-tripping and unknown-construct tolerance are trivial), and
//! a thin interpreter extracts the netlist-relevant pieces into the typed model.
//! Both stages report [`EdifError`] carrying byte offsets via [`tpt_io_core::ParseError`].

use alloc::string::{String, ToString};
use alloc::vec::Vec;

use tpt_io_core::{ParseError, Result, SpecReference};

use crate::error::EdifError;
use crate::model::{
    Cell, Design, Direction, Edif, Instance, Library, Net, Port, PortRef, Property, View, ViewType,
};
use crate::token::{tokenize, Located, Token};

/// A generic S-expression node.
#[derive(Debug, Clone, PartialEq)]
pub enum Sexp {
    /// Symbol / keyword atom.
    Sym(String),
    /// Number atom.
    Num(f64),
    /// String atom.
    Str(String),
    /// A parenthesized list.
    List(Vec<Sexp>),
}

impl Sexp {
    /// If this is a `List` whose first element is `Sym(head)`, return the tail.
    pub fn as_form(&self, head: &str) -> Option<&[Sexp]> {
        match self {
            Sexp::List(items) if !items.is_empty() => match &items[0] {
                Sexp::Sym(s) if s.eq_ignore_ascii_case(head) => Some(&items[1..]),
                _ => None,
            },
            _ => None,
        }
    }

    /// The head symbol of a list form, if any.
    pub fn head(&self) -> Option<&str> {
        match self {
            Sexp::List(items) => match items.first() {
                Some(Sexp::Sym(s)) => Some(s),
                _ => None,
            },
            _ => None,
        }
    }
}

/// A parser over a located token stream.
struct TokParser<'a> {
    toks: &'a [Located],
    pos: usize,
}

impl<'a> TokParser<'a> {
    fn new(toks: &'a [Located]) -> Self {
        Self { toks, pos: 0 }
    }

    fn peek(&self) -> Option<&Located> {
        self.toks.get(self.pos)
    }

    fn next(&mut self) -> Option<&Located> {
        let t = self.toks.get(self.pos);
        if t.is_some() {
            self.pos += 1;
        }
        t
    }

    fn parse_sexp(&mut self) -> Result<Sexp> {
        let tok = match self.next() {
            Some(t) => t.clone(),
            None => {
                let off = self.toks.last().map_or(0, |t| t.offset + 1);
                return Err(ParseError::eof(off));
            }
        };
        match tok.tok {
            Token::LParen => self.parse_list(tok.offset),
            Token::RParen => Err(ParseError::token(
                tok.offset,
                Some(alloc::borrow::Cow::Borrowed("atom or '(")),
                ")",
            )
            .with_spec(SpecReference::new("EDIF", "2 0 0 §3.2"))),
            Token::Symbol(s) => Ok(Sexp::Sym(s.clone())),
            Token::Number(n) => Ok(Sexp::Num(n)),
            Token::Str(s) => Ok(Sexp::Str(s.clone())),
        }
    }

    fn parse_list(&mut self, open_offset: usize) -> Result<Sexp> {
        let mut items = Vec::new();
        loop {
            match self.peek() {
                None => {
                    return Err(ParseError::eof(open_offset)
                        .with_spec(SpecReference::new("EDIF", "2 0 0 §3.2")));
                }
                Some(Located { tok: Token::RParen, .. }) => {
                    self.pos += 1;
                    return Ok(Sexp::List(items));
                }
                Some(_) => {
                    let e = self.parse_sexp()?;
                    items.push(e);
                }
            }
        }
    }
}

/// Parse a byte slice into the generic S-expression tree.
pub fn parse_sexp(input: &[u8]) -> core::result::Result<Vec<Sexp>, EdifError> {
    let toks = tokenize(input)?;
    let mut p = TokParser::new(&toks);
    let mut forms = Vec::new();
    while p.peek().is_some() {
        forms.push(p.parse_sexp()?);
    }
    Ok(forms)
}

/// An EDIF reader implementing [`tpt_io_core::Reader`].
#[derive(Debug, Clone, Default)]
pub struct EdifReader;

impl EdifReader {
    /// Create a new reader.
    pub fn new() -> Self {
        Self
    }
}

impl tpt_io_core::Reader<Edif> for EdifReader {
    type Error = EdifError;

    fn parse(&self, input: &[u8]) -> core::result::Result<Edif, EdifError> {
        parse_edif(input)
    }
}

/// Parse bytes into a typed [`Edif`] document.
pub fn parse_edif(input: &[u8]) -> core::result::Result<Edif, EdifError> {
    let forms = parse_sexp(input)?;
    // The document is the `(edif …)` form.
    let edif = forms
        .iter()
        .find(|f| f.head().is_some_and(|h| h.eq_ignore_ascii_case("edif")))
        .ok_or_else(|| {
            EdifError::Core(
                ParseError::custom(0, "no top-level (edif …) form found")
                    .with_spec(SpecReference::new("EDIF", "2 0 0 §4.1")),
            )
        })?;

    let tail = edif.as_form("edif").unwrap();
    let name = tail.first().and_then(symbol_str).map(str::to_string).unwrap_or_default();

    let version = read_version(tail).unwrap_or((2, 0, 0));
    let libraries = read_libraries(tail)?;
    let design = read_design(tail);

    Ok(Edif { name, version, libraries, design })
}

fn read_version(tail: &[Sexp]) -> Option<(u32, u32, u32)> {
    let v = tail.iter().find_map(|f| f.as_form("edifVersion"))?;
    let a = v.first().and_then(num_u32)?;
    let b = v.get(1).and_then(num_u32)?;
    let c = v.get(2).and_then(num_u32)?;
    Some((a, b, c))
}

fn read_libraries(tail: &[Sexp]) -> core::result::Result<Vec<Library>, EdifError> {
    let mut libs = Vec::new();
    for f in tail {
        if let Some(body) = f.as_form("library") {
            libs.push(read_library(body)?);
        }
    }
    Ok(libs)
}

fn read_library(body: &[Sexp]) -> core::result::Result<Library, EdifError> {
    let name = body.first().and_then(symbol_str).map(|s| s.to_string()).unwrap_or_default();
    let mut cells = Vec::new();
    for f in body {
        if let Some(cbody) = f.as_form("cell") {
            cells.push(read_cell(cbody)?);
        }
    }
    Ok(Library { name, cells })
}

fn read_cell(body: &[Sexp]) -> core::result::Result<Cell, EdifError> {
    let name = body.first().and_then(symbol_str).map(|s| s.to_string()).unwrap_or_default();
    let cell_type = body
        .iter()
        .find_map(|f| f.as_form("cellType"))
        .and_then(|t| t.first())
        .and_then(symbol_str)
        .map(String::from);
    let mut views = Vec::new();
    for f in body {
        if let Some(vbody) = f.as_form("view") {
            views.push(read_view(vbody)?);
        }
    }
    Ok(Cell { name, cell_type, views })
}

fn read_view(body: &[Sexp]) -> core::result::Result<View, EdifError> {
    let name = body.first().and_then(symbol_str).map(|s| s.to_string()).unwrap_or_default();
    let mut view_type = ViewType::Other(String::new());
    let mut interface = Vec::new();
    let mut instances = Vec::new();
    let mut nets = Vec::new();
    let mut properties = Vec::new();
    let mut geometry = Vec::new();

    for f in body {
        if let Some(t) = f.as_form("viewType") {
            if let Some(s) = t.first().and_then(symbol_str) {
                view_type = ViewType::parse(s);
            }
        } else if let Some(iface) = f.as_form("interface") {
            for port_form in iface {
                if let Some(pbody) = port_form.as_form("port") {
                    if let Some(p) = read_port(pbody) {
                        interface.push(p);
                    }
                }
            }
        } else if let Some(contents) = f.as_form("contents") {
            for c in contents {
                if let Some(ibody) = c.as_form("instance") {
                    if let Some(inst) = read_instance(ibody) {
                        instances.push(inst);
                    }
                } else if let Some(nbody) = c.as_form("net") {
                    if let Some(net) = read_net(nbody) {
                        nets.push(net);
                    }
                } else if let Some(pbody) = c.as_form("property") {
                    if let Some(p) = read_property(pbody) {
                        properties.push(p);
                    }
                } else if let Some(gbody) = c.as_form("geometry") {
                    // Figures may be grouped inside a `(geometry …)` container.
                    for g in gbody {
                        if let Some(fbody) = g.as_form("figure") {
                            if let Some(fig) = crate::geometry::read_figure(fbody) {
                                geometry.push(fig);
                            }
                        }
                    }
                } else if let Some(fbody) = c.as_form("figure") {
                    if let Some(fig) = crate::geometry::read_figure(fbody) {
                        geometry.push(fig);
                    }
                }
            }
        } else if let Some(pbody) = f.as_form("property") {
            if let Some(p) = read_property(pbody) {
                properties.push(p);
            }
        } else if let Some(fbody) = f.as_form("figure") {
            // A figure directly on the view (rare, but tolerated).
            if let Some(fig) = crate::geometry::read_figure(fbody) {
                geometry.push(fig);
            }
        }
    }

    Ok(View { name, view_type, interface, instances, nets, properties, geometry })
}

fn read_port(body: &[Sexp]) -> Option<Port> {
    let name = effective_name(body)?;
    let direction = body
        .iter()
        .find_map(|f| f.as_form("direction"))
        .and_then(|d| d.first())
        .and_then(symbol_str)
        .and_then(Direction::parse);
    Some(Port { name, direction })
}

fn read_instance(body: &[Sexp]) -> Option<Instance> {
    let name = effective_name(body)?;
    let mut cell_ref = String::new();
    let mut view_ref = String::new();
    let mut library_ref = String::new();
    for f in body {
        if let Some(c) = f.as_form("cellRef") {
            cell_ref = c.first().and_then(symbol_str).map(|s| s.to_string()).unwrap_or_default();
        } else if let Some(v) = f.as_form("viewRef") {
            view_ref = v.first().and_then(symbol_str).map(|s| s.to_string()).unwrap_or_default();
        } else if let Some(l) = f.as_form("libraryRef") {
            library_ref = l.first().and_then(symbol_str).map(|s| s.to_string()).unwrap_or_default();
        }
    }
    Some(Instance { name, cell_ref, view_ref, library_ref })
}

fn read_net(body: &[Sexp]) -> Option<Net> {
    let name = effective_name(body)?;
    let mut members = Vec::new();
    for f in body {
        if let Some(j) = f.as_form("joined") {
            for pr in j {
                if let Some(pbody) = pr.as_form("portRef") {
                    let port = pbody
                        .first()
                        .and_then(symbol_str)
                        .map(|s| s.to_string())
                        .unwrap_or_default();
                    let instance = pbody
                        .iter()
                        .find_map(|x| x.as_form("instanceRef"))
                        .and_then(|i| i.first())
                        .and_then(symbol_str)
                        .map(String::from);
                    members.push(PortRef { port, instance });
                }
            }
        } else if let Some(pr) = f.as_form("portRef") {
            // A net can also be declared as bare `(portRef …)` members.
            let port = pr.first().and_then(symbol_str).map(|s| s.to_string()).unwrap_or_default();
            let instance = pr
                .iter()
                .find_map(|x| x.as_form("instanceRef"))
                .and_then(|i| i.first())
                .and_then(symbol_str)
                .map(String::from);
            members.push(PortRef { port, instance });
        }
    }
    Some(Net { name, members })
}

pub(crate) fn read_property(body: &[Sexp]) -> Option<Property> {
    let key = body.first().and_then(symbol_str).map(str::to_string)?;
    // Value is usually the last atom (string/number/symbol).
    let value = body
        .last()
        .map(|v| match v {
            Sexp::Str(s) => s.clone(),
            Sexp::Sym(s) => s.clone(),
            Sexp::Num(n) => alloc::format!("{n}"),
            Sexp::List(_) => String::new(),
        })
        .unwrap_or_default();
    Some(Property { key, value })
}

fn read_design(tail: &[Sexp]) -> Option<Design> {
    let d = tail.iter().find_map(|f| f.as_form("design"))?;
    let name = d.first().and_then(symbol_str).map(|s| s.to_string()).unwrap_or_default();
    let mut cell_ref = String::new();
    let mut library_ref = String::new();
    for f in d {
        if let Some(c) = f.as_form("cellRef") {
            cell_ref = c.first().and_then(symbol_str).map(|s| s.to_string()).unwrap_or_default();
        } else if let Some(l) = f.as_form("libraryRef") {
            library_ref = l.first().and_then(symbol_str).map(|s| s.to_string()).unwrap_or_default();
        }
    }
    Some(Design { name, cell_ref, library_ref })
}

/// Resolve the effective (post-`rename`) name of a form.
///
/// EDIF wraps renamed identifiers as `(rename (portRef <orig>) <new>)` or
/// `(rename (netRef <orig>) <new>)`. If such a wrapper is present we use the
/// new name; otherwise we take the first atom of the form directly.
fn effective_name(body: &[Sexp]) -> Option<String> {
    // Direct name as first atom.
    if let Some(s) = body.first().and_then(symbol_str) {
        // If the first child is itself a `rename` form, prefer its new name.
        if let Some(r) = body.first().and_then(|f| f.as_form("rename")) {
            if let Some(new) = r.get(1).and_then(symbol_str) {
                return Some(new.to_string());
            }
        }
        return Some(s.to_string());
    }
    // Form itself is `(rename (…Ref <orig>) <new>)`.
    if let Some(r) = body.first().and_then(|f| f.as_form("rename")) {
        if let Some(new) = r.get(1).and_then(symbol_str) {
            return Some(new.to_string());
        }
        if let Some(orig) = r.first().and_then(|f| f.as_form("portRef")).or_else(|| {
            r.first().and_then(|f| {
                if let Sexp::List(inner) = f {
                    inner.first().and_then(|x| x.as_form("portRef"))
                } else {
                    None
                }
            })
        }) {
            return orig.first().and_then(symbol_str).map(String::from);
        }
    }
    None
}

fn symbol_str(s: &Sexp) -> Option<&str> {
    match s {
        Sexp::Sym(s) => Some(s),
        _ => None,
    }
}

fn num_u32(s: &Sexp) -> Option<u32> {
    match s {
        Sexp::Num(n) => {
            let r = *n;
            if r >= 0.0 && r.fract() == 0.0 {
                Some(r as u32)
            } else {
                None
            }
        }
        _ => None,
    }
}

/// EDIF syntactic/semantic validator.
#[derive(Debug, Clone, Default)]
pub struct EdifValidator;

impl EdifValidator {
    /// Create a new validator.
    pub fn new() -> Self {
        Self
    }
}

impl tpt_io_core::Validator<Edif> for EdifValidator {
    type Error = EdifError;

    fn validate_syntactic(&self, value: &Edif) -> core::result::Result<(), EdifError> {
        if value.name.is_empty() {
            return Err(EdifError::Core(
                ParseError::custom(0, "edif document missing a name")
                    .with_spec(SpecReference::new("EDIF", "2 0 0 §4.1")),
            ));
        }
        Ok(())
    }

    fn validate_semantic(&self, value: &Edif) -> core::result::Result<(), EdifError> {
        if let Some(design) = &value.design {
            if value.find_cell(&design.library_ref, &design.cell_ref).is_none() {
                return Err(EdifError::Core(
                    ParseError::custom(
                        0,
                        alloc::format!(
                            "design references undefined cell {} in library {}",
                            design.cell_ref,
                            design.library_ref
                        ),
                    )
                    .with_spec(SpecReference::new("EDIF", "2 0 0 §4.4")),
                ));
            }
        }
        Ok(())
    }
}
