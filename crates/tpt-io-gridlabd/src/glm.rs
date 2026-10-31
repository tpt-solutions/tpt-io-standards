//! Parser, model and writer for GLM files.

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::fmt::Write as _;

use tpt_io_core::ParseError;

use crate::error::GlmError;

/// A `name value` property. The value is the raw text up to the `;`.
pub type Property = (String, String);

/// A parsed GLM model: its top-level items in file order.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Glm {
    /// Top-level statements in file order.
    pub items: Vec<Item>,
}

/// One top-level statement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Item {
    /// A `#` or `@` directive, kept as written (for example `#set verbose=1`).
    Directive(String),
    /// `module name { … }`.
    Module { name: String, props: Vec<Property> },
    /// `clock { … }`.
    Clock { props: Vec<Property> },
    /// `class name { … }`.
    Class { name: String, props: Vec<Property> },
    /// `object class { … }`.
    Object(Object),
}

/// An `object` block, which may contain nested objects.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Object {
    /// The class name, e.g. `node` or `triplex_meter`.
    pub class: String,
    /// Properties in file order.
    pub props: Vec<Property>,
    /// Nested objects in file order.
    pub children: Vec<Object>,
}

impl Object {
    /// The `name` property, if set.
    pub fn name(&self) -> Option<&str> {
        self.prop("name")
    }

    /// The raw value of a property.
    pub fn prop(&self, key: &str) -> Option<&str> {
        self.props.iter().find(|(k, _)| k == key).map(|(_, v)| v.as_str())
    }
}

impl Glm {
    /// Top-level objects of the given class, e.g. `"node"`.
    pub fn objects_of_class<'a>(&'a self, class: &'a str) -> impl Iterator<Item = &'a Object> + 'a {
        self.items.iter().filter_map(move |item| match item {
            Item::Object(o) if o.class == class => Some(o),
            _ => None,
        })
    }

    /// Check that object names are unique and that every `parent` refers to a named object.
    pub fn validate(&self) -> Result<(), GlmError> {
        let mut names: Vec<&str> = Vec::new();
        let mut parents: Vec<&str> = Vec::new();
        for item in &self.items {
            if let Item::Object(o) = item {
                collect(o, &mut names, &mut parents);
            }
        }
        let mut sorted = names.clone();
        sorted.sort_unstable();
        if sorted.windows(2).any(|w| w[0] == w[1]) {
            return Err(GlmError::Invalid("duplicate object name".to_string()));
        }
        for p in parents {
            if !names.contains(&p) {
                return Err(GlmError::Invalid(alloc::format!("parent '{p}' is not a named object")));
            }
        }
        Ok(())
    }
}

fn collect<'a>(o: &'a Object, names: &mut Vec<&'a str>, parents: &mut Vec<&'a str>) {
    if let Some(n) = o.name() {
        names.push(n);
    }
    if let Some(p) = o.prop("parent") {
        parents.push(p);
    }
    for c in &o.children {
        collect(c, names, parents);
    }
}

/// Parse a GLM model.
pub fn parse(input: &[u8]) -> Result<Glm, GlmError> {
    let src = core::str::from_utf8(input)
        .map_err(|e| ParseError::custom(e.valid_up_to(), "model is not valid UTF-8"))?;
    let mut p = Parser { src, pos: 0 };
    let mut items = Vec::new();
    loop {
        p.skip_ws()?;
        if p.pos >= p.src.len() {
            break;
        }
        match p.peek() {
            Some(b'#' | b'@') => {
                let line = p.take_line();
                items.push(Item::Directive(line.trim().to_string()));
            }
            _ => {
                let at = p.pos;
                let head = p.word().ok_or_else(|| ParseError::custom(at, "expected a statement"))?;
                match head {
                    "object" => items.push(Item::Object(p.object()?)),
                    "module" => {
                        let (name, props) = p.named_block()?;
                        items.push(Item::Module { name, props });
                    }
                    "class" => {
                        let (name, props) = p.named_block()?;
                        items.push(Item::Class { name, props });
                    }
                    "clock" => {
                        let (_, props) = p.named_block()?;
                        items.push(Item::Clock { props });
                    }
                    _ => return Err(ParseError::custom(at, "unexpected top-level statement").into()),
                }
            }
        }
    }
    Ok(Glm { items })
}

struct Parser<'a> {
    src: &'a str,
    pos: usize,
}

impl<'a> Parser<'a> {
    fn peek(&self) -> Option<u8> {
        self.src.as_bytes().get(self.pos).copied()
    }

    fn peek_at(&self, off: usize) -> Option<u8> {
        self.src.as_bytes().get(self.pos + off).copied()
    }

    /// Skip whitespace, `//` line comments and `/* … */` block comments.
    fn skip_ws(&mut self) -> Result<(), ParseError> {
        loop {
            match self.peek() {
                Some(b' ' | b'\t' | b'\r' | b'\n') => self.pos += 1,
                Some(b'/') if self.peek_at(1) == Some(b'/') => {
                    self.take_line();
                }
                Some(b'/') if self.peek_at(1) == Some(b'*') => {
                    let rest = &self.src[self.pos + 2..];
                    match rest.find("*/") {
                        Some(i) => self.pos += 2 + i + 2,
                        None => return Err(ParseError::eof(self.src.len())),
                    }
                }
                _ => return Ok(()),
            }
        }
    }

    /// Return the rest of the current line (without the newline) and move past it.
    fn take_line(&mut self) -> &'a str {
        let start = self.pos;
        let end = match self.src[start..].find('\n') {
            Some(i) => start + i,
            None => self.src.len(),
        };
        self.pos = (end + 1).min(self.src.len());
        &self.src[start..end]
    }

    /// A bare word: anything up to whitespace or one of `{ } ;`.
    fn word(&mut self) -> Option<&'a str> {
        let start = self.pos;
        while let Some(c) = self.peek() {
            if matches!(c, b' ' | b'\t' | b'\r' | b'\n' | b'{' | b'}' | b';') {
                break;
            }
            self.pos += 1;
        }
        if self.pos == start {
            None
        } else {
            Some(&self.src[start..self.pos])
        }
    }

    /// The raw value of a property: everything up to `;` or `}` (not consumed).
    fn raw_value(&mut self) -> &'a str {
        let start = self.pos;
        while let Some(c) = self.peek() {
            if c == b';' || c == b'}' {
                break;
            }
            self.pos += 1;
        }
        self.src[start..self.pos].trim()
    }

    /// `[name] { body }` or `[name];` after a block keyword.
    fn named_block(&mut self) -> Result<(String, Vec<Property>), ParseError> {
        self.skip_ws()?;
        let name = match self.peek() {
            Some(b'{' | b';') | None => String::new(),
            _ => self.word().unwrap_or("").to_string(),
        };
        self.skip_ws()?;
        match self.peek() {
            Some(b';') => {
                self.pos += 1;
                Ok((name, Vec::new()))
            }
            Some(b'{') => {
                let at = self.pos;
                let (props, children) = self.body()?;
                if !children.is_empty() {
                    return Err(ParseError::custom(at, "objects are not allowed in this block"));
                }
                Ok((name, props))
            }
            _ => Err(ParseError::custom(self.pos, "expected '{' or ';'")),
        }
    }

    /// `class-name { … }` after the `object` keyword.
    fn object(&mut self) -> Result<Object, ParseError> {
        self.skip_ws()?;
        let class = self
            .word()
            .ok_or_else(|| ParseError::custom(self.pos, "expected an object class"))?
            .to_string();
        self.skip_ws()?;
        let (props, children) = match self.peek() {
            Some(b'{') => self.body()?,
            Some(b';') => {
                self.pos += 1;
                (Vec::new(), Vec::new())
            }
            _ => return Err(ParseError::custom(self.pos, "expected '{'")),
        };
        Ok(Object { class, props, children })
    }

    /// `{ … }`: properties and nested objects.
    fn body(&mut self) -> Result<(Vec<Property>, Vec<Object>), ParseError> {
        self.pos += 1;
        let mut props = Vec::new();
        let mut children = Vec::new();
        loop {
            self.skip_ws()?;
            match self.peek() {
                None => return Err(ParseError::eof(self.pos)),
                Some(b'}') => {
                    self.pos += 1;
                    return Ok((props, children));
                }
                Some(b';') => self.pos += 1,
                _ => {
                    let at = self.pos;
                    let key = self
                        .word()
                        .ok_or_else(|| ParseError::custom(at, "expected a property name"))?;
                    if key == "object" {
                        children.push(self.object()?);
                        continue;
                    }
                    self.skip_ws()?;
                    let value = self.raw_value();
                    props.push((key.to_string(), value.to_string()));
                    if self.peek() == Some(b';') {
                        self.pos += 1;
                    }
                }
            }
        }
    }
}

/// Serialise a model. Properties are written with their raw values, so parsing the
/// output gives back an equal model.
pub fn write(glm: &Glm) -> Vec<u8> {
    let mut out = String::new();
    for item in &glm.items {
        match item {
            Item::Directive(d) => {
                out.push_str(d);
                out.push('\n');
            }
            Item::Module { name, props } => block(&mut out, "module", name, props, &[], 0),
            Item::Clock { props } => block(&mut out, "clock", "", props, &[], 0),
            Item::Class { name, props } => block(&mut out, "class", name, props, &[], 0),
            Item::Object(o) => object(&mut out, o, 0),
        }
    }
    out.into_bytes()
}

fn object(out: &mut String, o: &Object, depth: usize) {
    block(out, "object", &o.class, &o.props, &o.children, depth);
}

fn block(out: &mut String, keyword: &str, name: &str, props: &[Property], children: &[Object], depth: usize) {
    let indent = "\t".repeat(depth);
    if name.is_empty() {
        let _ = writeln!(out, "{indent}{keyword} {{");
    } else {
        let _ = writeln!(out, "{indent}{keyword} {name} {{");
    }
    for (k, v) in props {
        let _ = writeln!(out, "{indent}\t{k} {v};");
    }
    for c in children {
        object(out, c, depth + 1);
    }
    let _ = writeln!(out, "{indent}}}");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nested_objects_and_comments() {
        let glm = parse(
            b"// model\nclock { timezone EST+5EDT; }\n#set verbose=1\n\
              object triplex_meter {\n  name tm1; /* note */\n  object house {\n    name h1;\n  }\n}\n",
        )
        .unwrap();
        assert_eq!(glm.items.len(), 3);
        let Item::Object(tm) = &glm.items[2] else { panic!("expected object") };
        assert_eq!(tm.name(), Some("tm1"));
        assert_eq!(tm.children[0].prop("name"), Some("h1"));
    }

    #[test]
    fn values_with_spaces_and_complex_numbers() {
        let glm = parse(b"object load { name l; constant_power_A 100+50j; startime '2000-01-01 0:00:00'; }").unwrap();
        let Item::Object(o) = &glm.items[0] else { panic!() };
        assert_eq!(o.prop("constant_power_A"), Some("100+50j"));
        assert_eq!(o.prop("startime"), Some("'2000-01-01 0:00:00'"));
    }

    #[test]
    fn unexpected_top_level_is_an_error() {
        assert!(parse(b"name n1;").is_err());
        assert!(parse(b"object node { name n1;").is_err());
    }
}
