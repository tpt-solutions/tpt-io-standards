//! A minimal, dependency-free JSON parser.
//!
//! GeoJSON (RFC 7946) is JSON, but the workspace philosophy is "pure Rust,
//! implemented from scratch" and `tpt-io-geo` is a foundational crate that
//! `tpt-io-citygml` depends on, so we avoid pulling in a heavyweight JSON
//! dependency. This module implements just enough of [RFC 8259] to parse and
//! serialise the GeoJSON grammar: objects, arrays, strings (with the standard
//! escape sequences and `\uXXXX` surrogates), numbers, and the literals
//! `true`/`false`/`null`.
//!
//! It is `no_std` compatible and only requires `alloc`.

use alloc::string::{String, ToString};
use alloc::vec::Vec;

/// A parsed JSON value.
#[derive(Debug, Clone, PartialEq)]
pub enum Json {
    /// A JSON object: an ordered list of key/value pairs.
    Object(Vec<(String, Json)>),
    /// A JSON array.
    Array(Vec<Json>),
    /// A JSON string.
    String(String),
    /// A JSON number, stored as `f64` (lossless enough for GeoJSON coordinates).
    Number(f64),
    /// A JSON boolean.
    Bool(bool),
    /// The JSON `null` literal.
    Null,
}

impl Json {
    /// Look up a member of an object by key.
    ///
    /// Returns `None` if this value is not an object or the key is absent.
    pub fn get(&self, key: &str) -> Option<&Json> {
        match self {
            Json::Object(entries) => entries.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }

    /// Returns the value as a string slice, if it is a string.
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Json::String(s) => Some(s),
            _ => None,
        }
    }

    /// Returns the value as an array slice, if it is an array.
    pub fn as_array(&self) -> Option<&[Json]> {
        match self {
            Json::Array(a) => Some(a),
            _ => None,
        }
    }

    /// Returns the value as an object entry slice, if it is an object.
    pub fn as_object(&self) -> Option<&[(String, Json)]> {
        match self {
            Json::Object(o) => Some(o),
            _ => None,
        }
    }

    /// Returns the value as an `f64`, if it is a number.
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Json::Number(n) => Some(*n),
            _ => None,
        }
    }

    /// Returns the value as a `bool`, if it is a boolean.
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Json::Bool(b) => Some(*b),
            _ => None,
        }
    }

    /// Whether this value is `null`.
    pub fn is_null(&self) -> bool {
        matches!(self, Json::Null)
    }
}

/// Error type for the JSON parser, carrying the byte offset of failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JsonError {
    /// Byte offset where the error occurred.
    pub offset: usize,
    /// Human-readable reason.
    pub reason: String,
}

impl JsonError {
    pub fn new(offset: usize, reason: &str) -> Self {
        Self { offset, reason: reason.to_string() }
    }
}

/// Parse a complete JSON document from a UTF-8 byte slice.
///
/// The entire input must be a single JSON value; trailing whitespace is
/// permitted but trailing non-whitespace tokens are rejected.
pub fn parse(input: &[u8]) -> Result<Json, JsonError> {
    let mut p = Parser { bytes: input, pos: 0 };
    p.skip_ws();
    let value = p.parse_value()?;
    p.skip_ws();
    if p.pos != p.bytes.len() {
        return Err(JsonError::new(p.pos, "trailing data after JSON value"));
    }
    Ok(value)
}

/// Parse a JSON value from a string slice.
pub fn parse_str(input: &str) -> Result<Json, JsonError> {
    parse(input.as_bytes())
}

struct Parser<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> Parser<'a> {
    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.pos).copied()
    }

    fn skip_ws(&mut self) {
        while let Some(b) = self.peek() {
            match b {
                b' ' | b'\t' | b'\n' | b'\r' => self.pos += 1,
                _ => break,
            }
        }
    }

    fn parse_value(&mut self) -> Result<Json, JsonError> {
        self.skip_ws();
        match self.peek() {
            Some(b'{') => self.parse_object(),
            Some(b'[') => self.parse_array(),
            Some(b'"') => Ok(Json::String(self.parse_string()?)),
            Some(b't') | Some(b'f') => self.parse_bool(),
            Some(b'n') => self.parse_null(),
            Some(b @ b'0'..=b'9') | Some(b @ b'-') => self.parse_number(b),
            Some(_) => Err(JsonError::new(self.pos, "unexpected token at value start")),
            None => Err(JsonError::new(self.pos, "unexpected end of input")),
        }
    }

    fn expect(&mut self, b: u8) -> Result<(), JsonError> {
        if self.peek() == Some(b) {
            self.pos += 1;
            Ok(())
        } else {
            Err(JsonError::new(self.pos, "expected structural token"))
        }
    }

    fn parse_object(&mut self) -> Result<Json, JsonError> {
        self.expect(b'{')?;
        let mut entries: Vec<(String, Json)> = Vec::new();
        self.skip_ws();
        if self.peek() == Some(b'}') {
            self.pos += 1;
            return Ok(Json::Object(entries));
        }
        loop {
            self.skip_ws();
            if self.peek() != Some(b'"') {
                return Err(JsonError::new(self.pos, "expected string key in object"));
            }
            let key = self.parse_string()?;
            self.skip_ws();
            self.expect(b':')?;
            let value = self.parse_value()?;
            entries.push((key, value));
            self.skip_ws();
            match self.peek() {
                Some(b',') => {
                    self.pos += 1;
                    continue;
                }
                Some(b'}') => {
                    self.pos += 1;
                    break;
                }
                _ => return Err(JsonError::new(self.pos, "expected ',' or '}' in object")),
            }
        }
        Ok(Json::Object(entries))
    }

    fn parse_array(&mut self) -> Result<Json, JsonError> {
        self.expect(b'[')?;
        let mut items: Vec<Json> = Vec::new();
        self.skip_ws();
        if self.peek() == Some(b']') {
            self.pos += 1;
            return Ok(Json::Array(items));
        }
        loop {
            let value = self.parse_value()?;
            items.push(value);
            self.skip_ws();
            match self.peek() {
                Some(b',') => {
                    self.pos += 1;
                    continue;
                }
                Some(b']') => {
                    self.pos += 1;
                    break;
                }
                _ => return Err(JsonError::new(self.pos, "expected ',' or ']' in array")),
            }
        }
        Ok(Json::Array(items))
    }

    fn parse_string(&mut self) -> Result<String, JsonError> {
        self.expect(b'"')?;
        let mut out = String::new();
        loop {
            let b = match self.peek() {
                Some(b) => b,
                None => return Err(JsonError::new(self.pos, "unterminated string")),
            };
            self.pos += 1;
            match b {
                b'"' => break,
                b'\\' => {
                    let esc = match self.peek() {
                        Some(e) => e,
                        None => return Err(JsonError::new(self.pos, "unterminated escape")),
                    };
                    self.pos += 1;
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
                            let c = self.parse_unicode_escape()?;
                            out.push(c);
                        }
                        _ => return Err(JsonError::new(self.pos, "invalid escape character")),
                    }
                }
                _ if b < 0x20 => {
                    return Err(JsonError::new(self.pos, "control character in string"));
                }
                _ => {
                    // Collect a UTF-8 sequence starting at `b`.
                    let len = utf8_len(b);
                    if len == 1 {
                        out.push(b as char);
                    } else {
                        let start = self.pos - 1;
                        if start + len > self.bytes.len() {
                            return Err(JsonError::new(self.pos, "truncated UTF-8"));
                        }
                        let s = core::str::from_utf8(&self.bytes[start..start + len])
                            .map_err(|_| JsonError::new(self.pos, "invalid UTF-8"))?;
                        out.push_str(s);
                        self.pos += len - 1;
                    }
                }
            }
        }
        Ok(out)
    }

    fn parse_unicode_escape(&mut self) -> Result<char, JsonError> {
        let hi = self.parse_hex4()?;
        if (0xD800..=0xDBFF).contains(&hi) {
            // High surrogate; expect a following low surrogate.
            if self.peek() == Some(b'\\') {
                self.pos += 1;
                if self.peek() == Some(b'u') {
                    self.pos += 1;
                    let lo = self.parse_hex4()?;
                    if (0xDC00..=0xDFFF).contains(&lo) {
                        let c = 0x10000 + ((hi - 0xD800) << 10) + (lo - 0xDC00);
                        return char::from_u32(c)
                            .ok_or_else(|| JsonError::new(self.pos, "bad surrogate"));
                    }
                }
            }
            return Err(JsonError::new(self.pos, "unpaired surrogate"));
        }
        char::from_u32(hi).ok_or_else(|| JsonError::new(self.pos, "invalid unicode scalar"))
    }

    fn parse_hex4(&mut self) -> Result<u32, JsonError> {
        if self.pos + 4 > self.bytes.len() {
            return Err(JsonError::new(self.pos, "truncated unicode escape"));
        }
        let mut val: u32 = 0;
        for _ in 0..4 {
            let b = self.bytes[self.pos];
            let digit = match b {
                b'0'..=b'9' => (b - b'0') as u32,
                b'a'..=b'f' => (b - b'a' + 10) as u32,
                b'A'..=b'F' => (b - b'A' + 10) as u32,
                _ => return Err(JsonError::new(self.pos, "bad hex digit")),
            };
            val = val * 16 + digit;
            self.pos += 1;
        }
        Ok(val)
    }

    fn parse_bool(&mut self) -> Result<Json, JsonError> {
        if self.bytes[self.pos..].starts_with(b"true") {
            self.pos += 4;
            Ok(Json::Bool(true))
        } else if self.bytes[self.pos..].starts_with(b"false") {
            self.pos += 5;
            Ok(Json::Bool(false))
        } else {
            Err(JsonError::new(self.pos, "invalid literal"))
        }
    }

    fn parse_null(&mut self) -> Result<Json, JsonError> {
        if self.bytes[self.pos..].starts_with(b"null") {
            self.pos += 4;
            Ok(Json::Null)
        } else {
            Err(JsonError::new(self.pos, "invalid literal"))
        }
    }

    fn parse_number(&mut self, first: u8) -> Result<Json, JsonError> {
        let start = self.pos;
        if first == b'-' {
            self.pos += 1;
        }
        // integer part
        if self.peek() == Some(b'0') {
            self.pos += 1;
        } else if matches!(self.peek(), Some(b'1'..=b'9')) {
            while matches!(self.peek(), Some(b'0'..=b'9')) {
                self.pos += 1;
            }
        } else {
            return Err(JsonError::new(start, "invalid number"));
        }
        // fraction
        if self.peek() == Some(b'.') {
            self.pos += 1;
            if !matches!(self.peek(), Some(b'0'..=b'9')) {
                return Err(JsonError::new(self.pos, "expected digits after '.'"));
            }
            while matches!(self.peek(), Some(b'0'..=b'9')) {
                self.pos += 1;
            }
        }
        // exponent
        if matches!(self.peek(), Some(b'e') | Some(b'E')) {
            self.pos += 1;
            if matches!(self.peek(), Some(b'+') | Some(b'-')) {
                self.pos += 1;
            }
            if !matches!(self.peek(), Some(b'0'..=b'9')) {
                return Err(JsonError::new(self.pos, "expected digits in exponent"));
            }
            while matches!(self.peek(), Some(b'0'..=b'9')) {
                self.pos += 1;
            }
        }
        let slice = &self.bytes[start..self.pos];
        let s =
            core::str::from_utf8(slice).map_err(|_| JsonError::new(start, "bad number bytes"))?;
        s.parse::<f64>().map(Json::Number).map_err(|_| JsonError::new(start, "number out of range"))
    }
}

fn utf8_len(b: u8) -> usize {
    if b < 0x80 {
        1
    } else if b >> 5 == 0b110 {
        2
    } else if b >> 4 == 0b1110 {
        3
    } else if b >> 3 == 0b11110 {
        4
    } else {
        1
    }
}

/// Serialise a [`Json`] value back to its compact textual form.
///
/// This is used for round-trip testing and for re-emitting GeoJSON.
pub fn to_string(value: &Json) -> String {
    let mut out = String::new();
    write_value(value, &mut out);
    out
}

fn write_value(v: &Json, out: &mut String) {
    match v {
        Json::Null => out.push_str("null"),
        Json::Bool(true) => out.push_str("true"),
        Json::Bool(false) => out.push_str("false"),
        Json::Number(n) => out.push_str(&format_number(*n)),
        Json::String(s) => write_string(s, out),
        Json::Array(items) => {
            out.push('[');
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write_value(item, out);
            }
            out.push(']');
        }
        Json::Object(entries) => {
            out.push('{');
            for (i, (k, val)) in entries.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write_string(k, out);
                out.push(':');
                write_value(val, out);
            }
            out.push('}');
        }
    }
}

fn format_number(n: f64) -> String {
    // Use a compact representation that round-trips for typical coordinates.
    let s = alloc::format!("{n}");
    s
}

fn write_string(s: &str, out: &mut String) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{0008}' => out.push_str("\\b"),
            '\u{000C}' => out.push_str("\\f"),
            c if (c as u32) < 0x20 => {
                out.push_str(&alloc::format!("\\u{:04x}", c as u32));
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_scalars() {
        assert_eq!(parse_str("null").unwrap(), Json::Null);
        assert_eq!(parse_str("true").unwrap(), Json::Bool(true));
        assert_eq!(parse_str("false").unwrap(), Json::Bool(false));
        assert_eq!(parse_str("3.5").unwrap(), Json::Number(3.5));
        assert_eq!(parse_str("-2").unwrap(), Json::Number(-2.0));
        assert_eq!(parse_str("1e3").unwrap(), Json::Number(1000.0));
    }

    #[test]
    fn parse_object_and_array() {
        let v = parse_str(r#"{"a":[1,2,"x"],"b":null}"#).unwrap();
        let obj = v.as_object().unwrap();
        assert_eq!(obj.len(), 2);
        assert_eq!(v.get("a").unwrap().as_array().unwrap().len(), 3);
    }

    #[test]
    fn parse_unicode_escape() {
        let v = parse_str(r#""caf\u00e9""#).unwrap();
        assert_eq!(v.as_str().unwrap(), "café");
    }

    #[test]
    fn parse_surrogate_pair() {
        let v = parse_str(r#""\uD83D\uDE00""#).unwrap();
        assert_eq!(v.as_str().unwrap(), "😀");
    }

    #[test]
    fn round_trip_string() {
        let v = parse_str(r#"{"x":1.25,"y":"a\nb"}"#).unwrap();
        let s = to_string(&v);
        let v2 = parse_str(&s).unwrap();
        assert_eq!(v, v2);
    }

    #[test]
    fn rejects_trailing_data() {
        assert!(parse_str(r#"1 2"#).is_err());
    }
}
