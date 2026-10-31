//! MathML expression parsing for SBML `<math>` elements.
//!
//! SBML encodes mathematical formulas as MathML (Content MathML) inside
//! `<math>` elements. This module builds a small expression AST ([`MathNode`])
//! and provides a parser driven by `quick-xml` over the surrounding
//! [`quick_xml::NsReader`].

use alloc::boxed::Box;
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use quick_xml::events::{BytesEnd, BytesStart, Event};
use quick_xml::NsReader;

use crate::error::SbmlError;

/// A parsed MathML (Content MathML) expression tree.
#[derive(Debug, Clone, PartialEq)]
pub enum MathNode {
    /// A variable reference (`<ci>name</ci>`).
    Ci(String),
    /// A numeric constant (`<cn>value</cn>`) with optional type/kind/units.
    Cn {
        /// MathML `type` attribute (e.g. `"integer"`, `"real"`, `"rational"`).
        type_: Option<String>,
        /// MathML `kind` (for `cn`) — rarely used; captured for completeness.
        kind: Option<String>,
        /// SBML `units` attribute attached to the constant.
        units: Option<String>,
        /// The numeric text content.
        value: String,
    },
    /// A leaf operator or symbol element (`<plus/>`, `<times/>`, `<csymbol>`…).
    Operator(String),
    /// A bound variable (`<bvar><ci>x</ci></bvar>`) — only meaningful inside [`MathNode::Lambda`].
    Bvar(String),
    /// A function application: `op` applied to `args`.
    Apply {
        /// The operator node (an [`MathNode::Operator`], [`MathNode::Ci`], or [`MathNode::Csymbol`]).
        op: Box<MathNode>,
        /// Argument expressions.
        args: Vec<MathNode>,
    },
    /// A lambda abstraction with bound variables and a body expression.
    Lambda {
        /// Bound variable names.
        bvars: Vec<String>,
        /// The body expression.
        body: Box<MathNode>,
    },
    /// A `<piece>` within a piecewise: `(value, condition)`.
    Piece {
        /// The value for this branch.
        value: Box<MathNode>,
        /// The condition under which the value applies.
        condition: Box<MathNode>,
    },
    /// An `<otherwise>` branch of a piecewise.
    Otherwise(Box<MathNode>),
    /// A `piecewise` expression.
    Piecewise {
        /// `(value, condition)` pairs.
        pieces: Vec<(MathNode, MathNode)>,
        /// Optional fallback branch.
        otherwise: Option<Box<MathNode>>,
    },
    /// A csymbol (with `definitionURL`) treated here as a named operator.
    Csymbol {
        /// `definitionURL` attribute.
        definition_url: Option<String>,
        /// Symbol text content.
        name: String,
    },
}

impl MathNode {
    /// Collect every `ci` identifier referenced anywhere in the tree (depth-first,
    /// in document order). Useful for reference-resolution validation.
    pub fn collect_references(&self, out: &mut Vec<String>) {
        match self {
            MathNode::Ci(name) => out.push(name.clone()),
            MathNode::Apply { op, args } => {
                op.collect_references(out);
                for a in args {
                    a.collect_references(out);
                }
            }
            MathNode::Lambda { bvars: _, body } => body.collect_references(out),
            MathNode::Piece { value, condition } => {
                value.collect_references(out);
                condition.collect_references(out);
            }
            MathNode::Otherwise(node) => node.collect_references(out),
            MathNode::Piecewise { pieces, otherwise } => {
                for (v, c) in pieces {
                    v.collect_references(out);
                    c.collect_references(out);
                }
                if let Some(o) = otherwise {
                    o.collect_references(out);
                }
            }
            MathNode::Csymbol { .. } => {}
            _ => {}
        }
    }

    /// Render the expression as a human-readable infix string (best-effort).
    pub fn to_infix_string(&self) -> String {
        fn join(args: &[MathNode], sep: &str) -> String {
            let mut s = String::new();
            for (i, a) in args.iter().enumerate() {
                if i > 0 {
                    s.push_str(sep);
                }
                s.push_str(&a.to_infix_string());
            }
            s
        }
        match self {
            MathNode::Ci(n) => n.clone(),
            MathNode::Cn { value, .. } => value.clone(),
            MathNode::Operator(o) => o.clone(),
            MathNode::Csymbol { name, .. } => name.clone(),
            MathNode::Bvar(n) => n.clone(),
            MathNode::Apply { op, args } => {
                let op_s = match &**op {
                    MathNode::Operator(o) => o.as_str(),
                    MathNode::Ci(c) => c.as_str(),
                    other => return other.to_infix_string(),
                };
                match op_s {
                    "plus" => join(args, " + "),
                    "minus" if args.len() == 1 => format!("-{}", args[0].to_infix_string()),
                    "minus" => join(args, " - "),
                    "times" => join(args, " * "),
                    "divide" => join(args, " / "),
                    "power" => format!(
                        "{} ^ {}",
                        args[0].to_infix_string(),
                        args.get(1).map(|a| a.to_infix_string()).unwrap_or_default()
                    ),
                    "root" => format!("root({})", join(args, ", ")),
                    "abs" => format!("abs({})", join(args, ", ")),
                    "exp" => format!("exp({})", join(args, ", ")),
                    "ln" => format!("ln({})", join(args, ", ")),
                    "log" => format!("log({})", join(args, ", ")),
                    "sin" => format!("sin({})", join(args, ", ")),
                    "cos" => format!("cos({})", join(args, ", ")),
                    "gt" => join(args, " > "),
                    "lt" => join(args, " < "),
                    "geq" => join(args, " >= "),
                    "leq" => join(args, " <= "),
                    "eq" => join(args, " == "),
                    "neq" => join(args, " != "),
                    "and" => join(args, " && "),
                    "or" => join(args, " || "),
                    "not" => format!(
                        "!({})",
                        args.first().map(|a| a.to_infix_string()).unwrap_or_default()
                    ),
                    "piecewise" => format!("piecewise({})", join(args, ", ")),
                    other => format!("{}({})", other, join(args, ", ")),
                }
            }
            MathNode::Lambda { bvars, body } => {
                format!("lambda({} : {})", bvars.join(", "), body.to_infix_string())
            }
            MathNode::Piece { value, condition } => {
                format!("{} if {}", value.to_infix_string(), condition.to_infix_string())
            }
            MathNode::Otherwise(node) => format!("else {}", node.to_infix_string()),
            MathNode::Piecewise { pieces, otherwise } => {
                let mut s = String::from("piecewise(");
                for (i, (v, c)) in pieces.iter().enumerate() {
                    if i > 0 {
                        s.push_str(", ");
                    }
                    s.push_str(&format!("{} if {}", v.to_infix_string(), c.to_infix_string()));
                }
                if let Some(o) = otherwise {
                    s.push_str(&format!(", else {}", o.to_infix_string()));
                }
                s.push(')');
                s
            }
        }
    }
}

fn ln_str(e: &BytesStart<'_>) -> String {
    String::from_utf8_lossy(e.local_name().as_ref()).into_owned()
}

fn ename(e: &BytesEnd<'_>) -> String {
    String::from_utf8_lossy(e.local_name().as_ref()).into_owned()
}

fn attr_opt(e: &BytesStart, key: &str) -> Option<String> {
    for a in e.attributes().flatten() {
        if core::str::from_utf8(a.key.local_name().as_ref()).ok()? == key {
            return Some(String::from_utf8_lossy(&a.value).into_owned());
        }
    }
    None
}

fn read_text_until(reader: &mut NsReader<&[u8]>, end: &str) -> Result<String, SbmlError> {
    let mut buf = Vec::new();
    let mut text = String::new();
    loop {
        match reader.read_event_into(&mut buf)? {
            Event::Text(t) => {
                let s: String = t.unescape()?.into_owned();
                text.push_str(&s);
            }
            Event::CData(c) => {
                text.push_str(&String::from_utf8_lossy(c.as_ref()));
            }
            Event::End(e) if ename(&e) == end => return Ok(text),
            Event::Eof => {
                return Err(SbmlError::MathParse(format!("unexpected EOF inside <{end}>")))
            }
            _ => {}
        }
    }
}

/// Consume events until the matching end tag `end` (handling arbitrary nesting).
pub(crate) fn skip_to_end(reader: &mut NsReader<&[u8]>, end: &str) -> Result<(), SbmlError> {
    let mut buf = Vec::new();
    let mut depth = 1usize;
    loop {
        match reader.read_event_into(&mut buf)? {
            Event::Start(e) if ln_str(&e) == end => depth += 1,
            Event::End(e) if ename(&e) == end => {
                depth -= 1;
                if depth == 0 {
                    return Ok(());
                }
            }
            Event::Eof => {
                return Err(SbmlError::MathParse(format!("unexpected EOF inside <{end}>")))
            }
            _ => {}
        }
    }
}

/// Parse a single MathML expression node. Returns `None` when the next event is
/// the end of the enclosing element (so callers can loop until `None`).
fn parse_node(reader: &mut NsReader<&[u8]>) -> Result<Option<MathNode>, SbmlError> {
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf)? {
            Event::Start(e) => {
                let name = ln_str(&e).to_string();
                match name.as_str() {
                    "ci" => {
                        let t = read_text_until(reader, "ci")?;
                        return Ok(Some(MathNode::Ci(t.trim().to_string())));
                    }
                    "cn" => {
                        let type_ = attr_opt(&e, "type");
                        let kind = attr_opt(&e, "kind");
                        let units = attr_opt(&e, "units");
                        let value = read_text_until(reader, "cn")?;
                        return Ok(Some(MathNode::Cn {
                            type_,
                            kind,
                            units,
                            value: value.trim().to_string(),
                        }));
                    }
                    "csymbol" => {
                        let definition_url = attr_opt(&e, "definitionURL");
                        let t = read_text_until(reader, "csymbol")?;
                        return Ok(Some(MathNode::Csymbol {
                            definition_url,
                            name: t.trim().to_string(),
                        }));
                    }
                    "apply" => return Ok(Some(parse_apply(reader)?)),
                    "lambda" => return Ok(Some(parse_lambda(reader)?)),
                    "piecewise" => return Ok(Some(parse_piecewise(reader)?)),
                    "semantics" => return Ok(Some(parse_semantics(reader)?)),
                    "piece" => {
                        let mut value = None;
                        let mut condition = None;
                        while let Some(n) = parse_node(reader)? {
                            match n {
                                MathNode::Ci(_)
                                | MathNode::Apply { .. }
                                | MathNode::Cn { .. }
                                | MathNode::Csymbol { .. } => {
                                    if value.is_none() {
                                        value = Some(Box::new(n));
                                    } else {
                                        condition = Some(Box::new(n));
                                    }
                                }
                                _ => {}
                            }
                        }
                        let value = value
                            .ok_or_else(|| SbmlError::MathParse("<piece> missing value".into()))?;
                        let condition = condition.ok_or_else(|| {
                            SbmlError::MathParse("<piece> missing condition".into())
                        })?;
                        return Ok(Some(MathNode::Piece { value, condition }));
                    }
                    "otherwise" => {
                        let mut body = None;
                        while let Some(n) = parse_node(reader)? {
                            body = Some(Box::new(n));
                        }
                        let body = body.ok_or_else(|| {
                            SbmlError::MathParse("<otherwise> missing expression".into())
                        })?;
                        return Ok(Some(MathNode::Otherwise(body)));
                    }
                    "bvar" => {
                        let mut name = None;
                        while let Some(n) = parse_node(reader)? {
                            if let MathNode::Ci(c) = n {
                                name = Some(c);
                            }
                        }
                        let name =
                            name.ok_or_else(|| SbmlError::MathParse("<bvar> missing <ci>".into()))?;
                        return Ok(Some(MathNode::Bvar(name)));
                    }
                    "true" | "false" => return Ok(Some(MathNode::Operator(name))),
                    other => {
                        skip_to_end(reader, other)?;
                        return Ok(Some(MathNode::Operator(name)));
                    }
                }
            }
            Event::Empty(e) => {
                return Ok(Some(MathNode::Operator(ln_str(&e).to_string())));
            }
            Event::Text(t) => {
                let _ = t.unescape()?;
            }
            Event::End(_) => return Ok(None),
            Event::Eof => return Err(SbmlError::MathParse("unexpected EOF in MathML".into())),
            _ => {}
        }
    }
}

fn parse_apply(reader: &mut NsReader<&[u8]>) -> Result<MathNode, SbmlError> {
    let mut nodes = Vec::new();
    while let Some(n) = parse_node(reader)? {
        nodes.push(n);
    }
    if nodes.is_empty() {
        return Err(SbmlError::MathParse("<apply> requires an operator".into()));
    }
    let op = Box::new(nodes.remove(0));
    Ok(MathNode::Apply { op, args: nodes })
}

fn parse_lambda(reader: &mut NsReader<&[u8]>) -> Result<MathNode, SbmlError> {
    let mut bvars = Vec::new();
    let mut body = None;
    while let Some(n) = parse_node(reader)? {
        match n {
            MathNode::Bvar(name) => bvars.push(name),
            other => body = Some(Box::new(other)),
        }
    }
    let body = body.ok_or_else(|| SbmlError::MathParse("<lambda> missing body".into()))?;
    Ok(MathNode::Lambda { bvars, body })
}

fn parse_piecewise(reader: &mut NsReader<&[u8]>) -> Result<MathNode, SbmlError> {
    let mut pieces = Vec::new();
    let mut otherwise = None;
    while let Some(n) = parse_node(reader)? {
        match n {
            MathNode::Piece { value, condition } => pieces.push((*value, *condition)),
            MathNode::Otherwise(node) => otherwise = Some(node),
            _ => {}
        }
    }
    Ok(MathNode::Piecewise { pieces, otherwise })
}

fn parse_semantics(reader: &mut NsReader<&[u8]>) -> Result<MathNode, SbmlError> {
    let mut first = None;
    while let Some(n) = parse_node(reader)? {
        if first.is_none() {
            first = Some(n);
        }
    }
    first.ok_or_else(|| SbmlError::MathParse("<semantics> missing expression".into()))
}

/// Parse the contents of a `<math>` element. The caller must have just read the
/// opening `<math>` [`Event::Start`]; this consumes the root expression and the
/// closing `</math>`.
pub(crate) fn parse_math(reader: &mut NsReader<&[u8]>) -> Result<MathNode, SbmlError> {
    let root = match parse_node(reader)? {
        Some(n) => n,
        None => return Err(SbmlError::MathParse("empty <math> element".into())),
    };
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf)? {
            Event::End(e) if ename(&e) == "math" => break,
            Event::Eof => return Err(SbmlError::MathParse("unexpected EOF in <math>".into())),
            _ => {}
        }
    }
    Ok(root)
}

/// Convenience: parse a MathML fragment from a string (used by tests).
#[cfg(feature = "std")]
pub fn parse_math_str(input: &str) -> Result<MathNode, SbmlError> {
    let mut reader = NsReader::from_str(input);
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf)? {
            Event::Start(e) if ln_str(&e) == "math" => return parse_math(&mut reader),
            Event::Eof => return Err(SbmlError::MathParse("no <math> element found".into())),
            _ => {}
        }
    }
}
