//! Minimal XML element reader and escaping helpers, shared by the XML-based format crates.
//!
//! The reader builds an owned [`Element`] tree from a complete document. It supports
//! elements, attributes (single or double quoted), character data, CDATA sections, the
//! five predefined entities and numeric character references. Comments, processing
//! instructions and `DOCTYPE` declarations are skipped. Namespace prefixes are kept in
//! [`Element::name`]; use [`Element::local_name`] to compare without the prefix.
//!
//! It is not a validating parser: DTDs are not expanded, external entities are not
//! resolved, and mismatched or unterminated tags are reported as errors.

use alloc::string::{String, ToString};
use alloc::vec::Vec;

use crate::error::{ParseError, Result};

/// An XML element with its attributes, children and direct character data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Element {
    /// The qualified name as written, e.g. `aixm:AirportHeliport`.
    pub name: String,
    /// Attributes in document order, with entities resolved.
    pub attributes: Vec<(String, String)>,
    /// Child elements in document order.
    pub children: Vec<Element>,
    /// Character data directly inside this element (CDATA included), concatenated.
    pub text: String,
    /// Byte offset of the start tag's `<`.
    pub offset: usize,
}

impl Element {
    /// The name without its namespace prefix (`aixm:Navaid` becomes `Navaid`).
    pub fn local_name(&self) -> &str {
        local(&self.name)
    }

    /// The value of the attribute whose local name is `name`.
    pub fn attr(&self, name: &str) -> Option<&str> {
        self.attributes.iter().find(|(k, _)| local(k) == name).map(|(_, v)| v.as_str())
    }

    /// The first child element whose local name is `name`.
    pub fn child(&self, name: &str) -> Option<&Element> {
        self.children.iter().find(|c| c.local_name() == name)
    }

    /// Every child element whose local name is `name`, in document order.
    pub fn children_named<'a>(&'a self, name: &'a str) -> impl Iterator<Item = &'a Element> + 'a {
        self.children.iter().filter(move |c| c.local_name() == name)
    }

    /// Follow a path of local names from this element, taking the first match at each step.
    pub fn path(&self, names: &[&str]) -> Option<&Element> {
        let mut cur = self;
        for name in names {
            cur = cur.child(name)?;
        }
        Some(cur)
    }

    /// Every descendant (not including `self`) whose local name is `name`, depth first.
    pub fn descendants_named(&self, name: &str) -> Vec<&Element> {
        let mut out = Vec::new();
        self.collect_descendants(name, &mut out);
        out
    }

    fn collect_descendants<'a>(&'a self, name: &str, out: &mut Vec<&'a Element>) {
        for c in &self.children {
            if c.local_name() == name {
                out.push(c);
            }
            c.collect_descendants(name, out);
        }
    }

    /// The character data with surrounding whitespace removed.
    pub fn text_trimmed(&self) -> &str {
        self.text.trim()
    }
}

/// The part of a qualified name after the last `:`.
pub fn local(name: &str) -> &str {
    name.rsplit(':').next().unwrap_or(name)
}

/// Parse a complete XML document and return its root element.
pub fn parse(input: &[u8]) -> Result<Element> {
    let mut p = Parser { input, pos: 0 };
    p.skip_misc()?;
    let root = p.element()?;
    p.skip_misc()?;
    if p.pos < input.len() {
        return Err(ParseError::custom(p.pos, "content after the root element"));
    }
    Ok(root)
}

/// Escape character data for use between tags (`&`, `<`, `>`).
pub fn escape_text(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            _ => out.push(c),
        }
    }
    out
}

/// Escape an attribute value for use inside double quotes.
pub fn escape_attr(s: &str) -> String {
    escape_text(s).replace('"', "&quot;")
}

/// Resolve the predefined entities and numeric character references in `s`.
fn unescape(s: &str, offset: usize) -> Result<String> {
    if !s.contains('&') {
        return Ok(s.to_string());
    }
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(i) = rest.find('&') {
        out.push_str(&rest[..i]);
        let after = &rest[i + 1..];
        let end = after
            .find(';')
            .ok_or_else(|| ParseError::custom(offset, "unterminated entity reference"))?;
        let name = &after[..end];
        let ch = match name {
            "lt" => '<',
            "gt" => '>',
            "amp" => '&',
            "quot" => '"',
            "apos" => '\'',
            _ if name.starts_with("#x") || name.starts_with("#X") => {
                char_ref(&name[2..], 16, offset)?
            }
            _ if name.starts_with('#') => char_ref(&name[1..], 10, offset)?,
            _ => return Err(ParseError::custom(offset, "unknown entity reference")),
        };
        out.push(ch);
        rest = &after[end + 1..];
    }
    out.push_str(rest);
    Ok(out)
}

fn char_ref(digits: &str, radix: u32, offset: usize) -> Result<char> {
    u32::from_str_radix(digits, radix)
        .ok()
        .and_then(char::from_u32)
        .ok_or_else(|| ParseError::custom(offset, "invalid character reference"))
}

struct Parser<'a> {
    input: &'a [u8],
    pos: usize,
}

impl<'a> Parser<'a> {
    fn starts_with(&self, s: &str) -> bool {
        self.input[self.pos..].starts_with(s.as_bytes())
    }

    fn skip_ws(&mut self) {
        while self.pos < self.input.len() && self.input[self.pos].is_ascii_whitespace() {
            self.pos += 1;
        }
    }

    /// Skip whitespace, `<?…?>`, `<!--…-->` and `<!DOCTYPE …>` (with any internal subset).
    fn skip_misc(&mut self) -> Result<()> {
        loop {
            self.skip_ws();
            if self.starts_with("<?") {
                self.skip_until("?>")?;
            } else if self.starts_with("<!--") {
                self.skip_until("-->")?;
            } else if self.starts_with("<!DOCTYPE") {
                self.skip_doctype()?;
            } else {
                return Ok(());
            }
        }
    }

    fn skip_until(&mut self, end: &str) -> Result<()> {
        let at = find(&self.input[self.pos..], end.as_bytes())
            .ok_or_else(|| ParseError::eof(self.input.len()))?;
        self.pos += at + end.len();
        Ok(())
    }

    fn skip_doctype(&mut self) -> Result<()> {
        let mut depth = 0usize;
        while self.pos < self.input.len() {
            match self.input[self.pos] {
                b'[' => depth += 1,
                b']' => depth = depth.saturating_sub(1),
                b'>' if depth == 0 => {
                    self.pos += 1;
                    return Ok(());
                }
                _ => {}
            }
            self.pos += 1;
        }
        Err(ParseError::eof(self.input.len()))
    }

    fn name(&mut self) -> Result<String> {
        let start = self.pos;
        while self.pos < self.input.len() {
            let b = self.input[self.pos];
            if b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-' | b'.' | b':') || b >= 0x80 {
                self.pos += 1;
            } else {
                break;
            }
        }
        if self.pos == start {
            return Err(ParseError::invalid_byte(
                start,
                self.input.get(start).copied().unwrap_or(0),
            ));
        }
        core::str::from_utf8(&self.input[start..self.pos])
            .map(|s| s.to_string())
            .map_err(|_| ParseError::custom(start, "invalid UTF-8 in name"))
    }

    fn element(&mut self) -> Result<Element> {
        let offset = self.pos;
        if self.input.get(self.pos) != Some(&b'<') {
            return Err(ParseError::custom(self.pos, "expected '<'"));
        }
        self.pos += 1;
        let name = self.name()?;
        let mut attributes = Vec::new();
        loop {
            self.skip_ws();
            match self.input.get(self.pos) {
                Some(b'/') => {
                    if self.input.get(self.pos + 1) != Some(&b'>') {
                        return Err(ParseError::custom(self.pos, "expected '/>'"));
                    }
                    self.pos += 2;
                    return Ok(Element {
                        name,
                        attributes,
                        children: Vec::new(),
                        text: String::new(),
                        offset,
                    });
                }
                Some(b'>') => {
                    self.pos += 1;
                    break;
                }
                Some(_) => {
                    let key = self.name()?;
                    self.skip_ws();
                    if self.input.get(self.pos) != Some(&b'=') {
                        return Err(ParseError::custom(
                            self.pos,
                            "expected '=' after attribute name",
                        ));
                    }
                    self.pos += 1;
                    self.skip_ws();
                    let value = self.attr_value()?;
                    attributes.push((key, value));
                }
                None => return Err(ParseError::eof(self.pos)),
            }
        }

        let mut children = Vec::new();
        let mut text = String::new();
        loop {
            if self.pos >= self.input.len() {
                return Err(ParseError::eof(self.pos));
            }
            if self.starts_with("</") {
                self.pos += 2;
                let close = self.name()?;
                self.skip_ws();
                if self.input.get(self.pos) != Some(&b'>') {
                    return Err(ParseError::custom(self.pos, "expected '>' in end tag"));
                }
                self.pos += 1;
                if close != name {
                    return Err(ParseError::custom(self.pos, "mismatched end tag"));
                }
                return Ok(Element { name, attributes, children, text, offset });
            } else if self.starts_with("<!--") {
                self.skip_until("-->")?;
            } else if self.starts_with("<![CDATA[") {
                self.pos += 9;
                let end = find(&self.input[self.pos..], b"]]>")
                    .ok_or_else(|| ParseError::eof(self.input.len()))?;
                let raw = core::str::from_utf8(&self.input[self.pos..self.pos + end])
                    .map_err(|_| ParseError::custom(self.pos, "invalid UTF-8 in CDATA"))?;
                text.push_str(raw);
                self.pos += end + 3;
            } else if self.starts_with("<?") {
                self.skip_until("?>")?;
            } else if self.input[self.pos] == b'<' {
                children.push(self.element()?);
            } else {
                let start = self.pos;
                while self.pos < self.input.len() && self.input[self.pos] != b'<' {
                    self.pos += 1;
                }
                let raw = core::str::from_utf8(&self.input[start..self.pos])
                    .map_err(|_| ParseError::custom(start, "invalid UTF-8 in text"))?;
                text.push_str(&unescape(raw, start)?);
            }
        }
    }

    fn attr_value(&mut self) -> Result<String> {
        let quote = match self.input.get(self.pos) {
            Some(&q @ (b'"' | b'\'')) => q,
            _ => return Err(ParseError::custom(self.pos, "expected quoted attribute value")),
        };
        self.pos += 1;
        let start = self.pos;
        while self.pos < self.input.len() && self.input[self.pos] != quote {
            if self.input[self.pos] == b'<' {
                return Err(ParseError::custom(self.pos, "'<' inside attribute value"));
            }
            self.pos += 1;
        }
        if self.pos >= self.input.len() {
            return Err(ParseError::eof(self.pos));
        }
        let raw = core::str::from_utf8(&self.input[start..self.pos])
            .map_err(|_| ParseError::custom(start, "invalid UTF-8 in attribute"))?;
        self.pos += 1;
        unescape(raw, start)
    }
}

fn find(hay: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || hay.len() < needle.len() {
        return None;
    }
    hay.windows(needle.len()).position(|w| w == needle)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_nested_elements_attributes_and_text() {
        let doc = parse(
            br#"<?xml version="1.0"?>
            <!-- c --><root a="1" b='x &amp; y'><item id="7">hi &lt;there&gt;</item><empty/></root>"#,
        )
        .unwrap();
        assert_eq!(doc.name, "root");
        assert_eq!(doc.attr("b"), Some("x & y"));
        let item = doc.child("item").unwrap();
        assert_eq!(item.attr("id"), Some("7"));
        assert_eq!(item.text, "hi <there>");
        assert_eq!(doc.children.len(), 2);
    }

    #[test]
    fn namespaces_cdata_and_paths() {
        let doc = parse(b"<a:top xmlns:a='u'><a:mid><![CDATA[1 < 2]]></a:mid></a:top>").unwrap();
        assert_eq!(doc.local_name(), "top");
        assert_eq!(doc.path(&["mid"]).unwrap().text, "1 < 2");
        assert_eq!(doc.descendants_named("mid").len(), 1);
    }

    #[test]
    fn rejects_malformed_documents() {
        assert!(parse(b"<a><b></a>").is_err());
        assert!(parse(b"<a>").is_err());
        assert!(parse(b"<a/>trailing").is_err());
        assert!(parse(b"<a x=1/>").is_err());
        assert!(parse(b"<a>&bogus;</a>").is_err());
    }

    #[test]
    fn escaping_round_trips() {
        let s = "a < b & \"c\"";
        let text = alloc::format!("<t v=\"{}\">{}</t>", escape_attr(s), escape_text(s));
        let doc = parse(text.as_bytes()).unwrap();
        assert_eq!(doc.attr("v"), Some(s));
        assert_eq!(doc.text, s);
    }
}
