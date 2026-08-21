//! A minimal, dependency-free JSON reader (RFC 8259).
//!
//! GeoJSON is JSON, and several other workspace formats (e.g. FHIR) also use
//! JSON, so this small recursive-descent parser is shared. It targets
//! `no_std` + `alloc` and reports byte offsets through
//! [`tpt_io_core::ParseError`] for precise error messages.
//!
//! Supported: objects, arrays, strings (with `\u` escapes and surrogate
//! pairs), numbers, `true`/`false`/`null`. Object members are collected into a
//! [`BTreeMap`]; duplicate keys keep the last value.

use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

use crate::{ParseError, Result, SpecReference};

/// A parsed JSON value.
#[derive(Debug, Clone, PartialEq)]
pub enum Json {
    /// `null`.
    Null,
    /// Boolean.
    Bool(bool),
    /// Number (all JSON numbers are stored as `f64`).
    Num(f64),
    /// String.
    Str(String),
    /// Array.
    Array(Vec<Json>),
    /// Object (member order is not preserved).
    Object(BTreeMap<String, Json>),
}

impl Json {
    /// If this is an `Object`, return a member by key.
    pub fn get(&self, key: &str) -> Option<&Json> {
        match self {
            Json::Object(m) => m.get(key),
            _ => None,
        }
    }

    /// Convenience: this value as a string slice, if it is a `Str`.
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Json::Str(s) => Some(s),
            _ => None,
        }
    }

    /// Convenience: this value as an `f64`, if it is a `Num`.
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Json::Num(n) => Some(*n),
            _ => None,
        }
    }

    /// Convenience: this value as a `bool`, if it is a `Bool`.
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Json::Bool(b) => Some(*b),
            _ => None,
        }
    }

    /// Convenience: this value as an array slice, if it is an `Array`.
    pub fn as_array(&self) -> Option<&[Json]> {
        match self {
            Json::Array(a) => Some(a),
            _ => None,
        }
    }
}

struct Parser<'a> {
    b: &'a [u8],
    i: usize,
}

impl<'a> Parser<'a> {
    fn peek(&self) -> Option<u8> {
        self.b.get(self.i).copied()
    }

    fn bump(&mut self) -> Option<u8> {
        let c = self.b.get(self.i).copied();
        if c.is_some() {
            self.i += 1;
        }
        c
    }

    fn skip_ws(&mut self) {
        while let Some(c) = self.peek() {
            if c.is_ascii_whitespace() {
                self.i += 1;
            } else {
                break;
            }
        }
    }

    fn parse(&mut self) -> Result<Json> {
        self.skip_ws();
        match self.peek() {
            Some(b'{') => self.parse_object(),
            Some(b'[') => self.parse_array(),
            Some(b'"') => Ok(Json::Str(self.parse_string()?)),
            Some(b't') | Some(b'f') | Some(b'n') => self.parse_literal(),
            Some(c) if c == b'-' || c.is_ascii_digit() => self.parse_number(),
            Some(c) => Err(ParseError::token(
                self.i,
                Some(alloc::borrow::Cow::Borrowed("value")),
                alloc::format!("unexpected byte 0x{c:02x}"),
            )
            .with_spec(SpecReference::new("JSON", "RFC 8259 §2"))),
            None => {
                Err(ParseError::eof(self.i).with_spec(SpecReference::new("JSON", "RFC 8259 §2")))
            }
        }
    }

    fn expect(&mut self, byte: u8) -> Result<()> {
        match self.bump() {
            Some(c) if c == byte => Ok(()),
            Some(c) => Err(ParseError::token(
                self.i - 1,
                Some(alloc::borrow::Cow::Owned(alloc::format!("{}", byte as char))),
                alloc::format!("0x{c:02x}"),
            )
            .with_spec(SpecReference::new("JSON", "RFC 8259 §2"))),
            None => {
                Err(ParseError::eof(self.i).with_spec(SpecReference::new("JSON", "RFC 8259 §2")))
            }
        }
    }

    fn parse_object(&mut self) -> Result<Json> {
        self.expect(b'{')?;
        let mut map = BTreeMap::new();
        self.skip_ws();
        if self.peek() == Some(b'}') {
            self.bump();
            return Ok(Json::Object(map));
        }
        loop {
            self.skip_ws();
            if self.peek() != Some(b'"') {
                return Err(ParseError::token(
                    self.i,
                    Some(alloc::borrow::Cow::Borrowed("object key")),
                    alloc::format!("0x{:02x}", self.peek().unwrap_or(0)),
                )
                .with_spec(SpecReference::new("JSON", "RFC 8259 §2")));
            }
            let key = self.parse_string()?;
            self.skip_ws();
            self.expect(b':')?;
            let value = self.parse()?;
            map.insert(key, value);
            self.skip_ws();
            match self.bump() {
                Some(b',') => continue,
                Some(b'}') => break,
                Some(c) => {
                    return Err(ParseError::token(
                        self.i - 1,
                        Some(alloc::borrow::Cow::Borrowed("',' or '}'")),
                        alloc::format!("0x{c:02x}"),
                    )
                    .with_spec(SpecReference::new("JSON", "RFC 8259 §2")))
                }
                None => {
                    return Err(ParseError::eof(self.i)
                        .with_spec(SpecReference::new("JSON", "RFC 8259 §2")))
                }
            }
        }
        Ok(Json::Object(map))
    }

    fn parse_array(&mut self) -> Result<Json> {
        self.expect(b'[')?;
        let mut items = Vec::new();
        self.skip_ws();
        if self.peek() == Some(b']') {
            self.bump();
            return Ok(Json::Array(items));
        }
        loop {
            let value = self.parse()?;
            items.push(value);
            self.skip_ws();
            match self.bump() {
                Some(b',') => continue,
                Some(b']') => break,
                Some(c) => {
                    return Err(ParseError::token(
                        self.i - 1,
                        Some(alloc::borrow::Cow::Borrowed("',' or ']'")),
                        alloc::format!("0x{c:02x}"),
                    )
                    .with_spec(SpecReference::new("JSON", "RFC 8259 §2")))
                }
                None => {
                    return Err(ParseError::eof(self.i)
                        .with_spec(SpecReference::new("JSON", "RFC 8259 §2")))
                }
            }
        }
        Ok(Json::Array(items))
    }

    fn parse_string(&mut self) -> Result<String> {
        self.expect(b'"')?;
        let mut out = String::new();
        loop {
            let c = match self.bump() {
                Some(c) => c,
                None => {
                    return Err(ParseError::eof(self.i)
                        .with_spec(SpecReference::new("JSON", "RFC 8259 §2")))
                }
            };
            match c {
                b'"' => break,
                b'\\' => {
                    let esc = match self.bump() {
                        Some(e) => e,
                        None => {
                            return Err(ParseError::eof(self.i)
                                .with_spec(SpecReference::new("JSON", "RFC 8259 §2")))
                        }
                    };
                    match esc {
                        b'"' => out.push('"'),
                        b'\\' => out.push('\\'),
                        b'/' => out.push('/'),
                        b'b' => out.push('\u{0008}'),
                        b'f' => out.push('\u{000C}'),
                        b'n' => out.push('\n'),
                        b'r' => out.push('\r'),
                        b't' => out.push('\t'),
                        b'u' => {
                            let cp = self.parse_hex4()?;
                            if (0xD800..=0xDBFF).contains(&cp) {
                                // High surrogate: expect a following low surrogate.
                                if self.bump() != Some(b'\\') || self.bump() != Some(b'u') {
                                    return Err(ParseError::token(
                                        self.i,
                                        Some(alloc::borrow::Cow::Borrowed("low surrogate")),
                                        "end of escape",
                                    )
                                    .with_spec(SpecReference::new("JSON", "RFC 8259 §7")));
                                }
                                let low = self.parse_hex4()?;
                                if !(0xDC00..=0xDFFF).contains(&low) {
                                    return Err(ParseError::token(
                                        self.i,
                                        Some(alloc::borrow::Cow::Borrowed("low surrogate")),
                                        alloc::format!("0x{low:04x}"),
                                    )
                                    .with_spec(SpecReference::new("JSON", "RFC 8259 §7")));
                                }
                                let c = 0x10000 + ((cp - 0xD800) << 10) + (low - 0xDC00);
                                match char::from_u32(c) {
                                    Some(ch) => out.push(ch),
                                    None => out.push('\u{FFFD}'),
                                }
                            } else if (0xDC00..=0xDFFF).contains(&cp) {
                                return Err(ParseError::token(
                                    self.i,
                                    Some(alloc::borrow::Cow::Borrowed("unexpected low surrogate")),
                                    alloc::format!("0x{cp:04x}"),
                                )
                                .with_spec(SpecReference::new("JSON", "RFC 8259 §7")));
                            } else {
                                match char::from_u32(cp) {
                                    Some(ch) => out.push(ch),
                                    None => out.push('\u{FFFD}'),
                                }
                            }
                        }
                        other => {
                            return Err(ParseError::token(
                                self.i - 1,
                                Some(alloc::borrow::Cow::Borrowed("escape")),
                                alloc::format!("0x{other:02x}"),
                            )
                            .with_spec(SpecReference::new("JSON", "RFC 8259 §7")))
                        }
                    }
                }
                _ => {
                    // Raw UTF-8 byte; collect the full code point.
                    if c < 0x80 {
                        out.push(c as char);
                    } else {
                        let start = self.i - 1;
                        let len = if c >= 0xF0 {
                            4
                        } else if c >= 0xE0 {
                            3
                        } else {
                            2
                        };
                        let slice = self.b.get(start..start + len).ok_or_else(|| {
                            ParseError::eof(self.i)
                                .with_spec(SpecReference::new("JSON", "RFC 8259 §8.1"))
                        })?;
                        match core::str::from_utf8(slice) {
                            Ok(s) => out.push_str(s),
                            Err(_) => out.push('\u{FFFD}'),
                        }
                        self.i = start + len;
                    }
                }
            }
        }
        Ok(out)
    }

    fn parse_hex4(&mut self) -> Result<u32> {
        let mut val = 0u32;
        for _ in 0..4 {
            let c = self.bump().ok_or_else(|| {
                ParseError::eof(self.i).with_spec(SpecReference::new("JSON", "RFC 8259 §7"))
            })?;
            let d = match c {
                b'0'..=b'9' => (c - b'0') as u32,
                b'a'..=b'f' => (c - b'a' + 10) as u32,
                b'A'..=b'F' => (c - b'A' + 10) as u32,
                _ => {
                    return Err(ParseError::token(
                        self.i - 1,
                        Some(alloc::borrow::Cow::Borrowed("hex digit")),
                        alloc::format!("0x{c:02x}"),
                    )
                    .with_spec(SpecReference::new("JSON", "RFC 8259 §7")))
                }
            };
            val = (val << 4) | d;
        }
        Ok(val)
    }

    fn parse_literal(&mut self) -> Result<Json> {
        if self.b.get(self.i..self.i + 4) == Some(b"true") {
            self.i += 4;
            Ok(Json::Bool(true))
        } else if self.b.get(self.i..self.i + 5) == Some(b"false") {
            self.i += 5;
            Ok(Json::Bool(false))
        } else if self.b.get(self.i..self.i + 4) == Some(b"null") {
            self.i += 4;
            Ok(Json::Null)
        } else {
            Err(ParseError::token(
                self.i,
                Some(alloc::borrow::Cow::Borrowed("true/false/null")),
                alloc::format!("0x{:02x}", self.peek().unwrap_or(0)),
            )
            .with_spec(SpecReference::new("JSON", "RFC 8259 §3")))
        }
    }

    fn parse_number(&mut self) -> Result<Json> {
        let start = self.i;
        if self.peek() == Some(b'-') {
            self.i += 1;
        }
        while let Some(c) = self.peek() {
            match c {
                b'0'..=b'9' | b'.' | b'e' | b'E' | b'+' | b'-' => self.i += 1,
                _ => break,
            }
        }
        let slice = self.b.get(start..self.i).ok_or_else(|| {
            ParseError::eof(self.i).with_spec(SpecReference::new("JSON", "RFC 8259 §6"))
        })?;
        let s = core::str::from_utf8(slice).map_err(|_| {
            ParseError::token(start, Some(alloc::borrow::Cow::Borrowed("number")), "invalid utf8")
                .with_spec(SpecReference::new("JSON", "RFC 8259 §6"))
        })?;
        s.parse::<f64>().map(Json::Num).map_err(|_| {
            ParseError::token(start, Some(alloc::borrow::Cow::Borrowed("number")), s.to_string())
                .with_spec(SpecReference::new("JSON", "RFC 8259 §6"))
        })
    }
}

/// Parse a JSON document, returning the root value.
pub fn parse_json(input: &[u8]) -> Result<Json> {
    let mut p = Parser { b: input, i: 0 };
    let root = p.parse()?;
    p.skip_ws();
    if p.peek().is_some() {
        return Err(ParseError::token(
            p.i,
            Some(alloc::borrow::Cow::Borrowed("end of input")),
            alloc::format!("0x{:02x}", p.peek().unwrap_or(0)),
        )
        .with_spec(SpecReference::new("JSON", "RFC 8259 §2")));
    }
    Ok(root)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_primitives() {
        assert_eq!(parse_json(b"null").unwrap(), Json::Null);
        assert_eq!(parse_json(b"true").unwrap(), Json::Bool(true));
        assert_eq!(parse_json(b"  -12.5e2 ").unwrap(), Json::Num(-1250.0));
        assert_eq!(parse_json(b"\"hi\"").unwrap(), Json::Str("hi".into()));
    }

    #[test]
    fn parses_object_and_array() {
        let j = parse_json(br#"{"a":[1,2,"x"],"b":null}"#).unwrap();
        let m = match j {
            Json::Object(m) => m,
            _ => panic!("expected object"),
        };
        assert_eq!(
            m.get("a"),
            Some(&Json::Array(vec![Json::Num(1.0), Json::Num(2.0), Json::Str("x".into())]))
        );
        assert_eq!(m.get("b"), Some(&Json::Null));
    }

    #[test]
    fn parses_unicode_escapes() {
        assert_eq!(parse_json(br#""\u00e9""#).unwrap(), Json::Str("é".into()));
        assert_eq!(parse_json(br#""\uD83D\uDE00""#).unwrap(), Json::Str("😀".into()));
    }

    #[test]
    fn rejects_trailing_garbage() {
        assert!(parse_json(b"1 2").is_err());
    }
}
