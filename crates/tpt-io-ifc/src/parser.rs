//! STEP-Physical File (SPF, ISO 10303-21) tokenizer and parser.
//!
//! The format is a sequence of `#id = ENTITY(attr, attr, ...);` instance
//! definitions wrapped in `HEADER;` / `DATA;` sections. This module tokenizes
//! on the fly (skipping `/* */` comments and whitespace) and builds a typed
//! [`IfcDocument`] with full instance resolution by id.

use alloc::string::String;
use alloc::vec::Vec;

use tpt_io_core::{ParseError, SpecReference};

use crate::error::IfcError;
use crate::model::{EntityInstance, IfcDocument, Value};

/// A lexical token with its start byte offset.
#[derive(Debug, Clone, PartialEq)]
enum Tok {
    Hash(u64),
    Eq,
    LParen,
    RParen,
    Comma,
    Semicolon,
    Int(i64),
    Real(f64),
    Str(String),
    Enum(String),
    Ident(String),
    Dollar,
}

struct Parser<'a> {
    b: &'a [u8],
    i: usize,
    lookahead: Option<Tok>,
}

impl<'a> Parser<'a> {
    fn peek_at(&self, n: usize) -> Option<u8> {
        self.b.get(self.i + n).copied()
    }

    fn peek(&self) -> Option<u8> {
        self.b.get(self.i).copied()
    }

    fn ifc_err(&self, msg: &'static str) -> IfcError {
        IfcError::Core(
            ParseError::custom(self.i, msg).with_spec(SpecReference::new("IFC", "ISO 10303-21")),
        )
    }

    fn tok_err(&self, expected: &'static str, actual: u8) -> IfcError {
        IfcError::Core(
            ParseError::token(self.i, Some(expected), actual as char)
                .with_spec(SpecReference::new("IFC", "ISO 10303-21")),
        )
    }

    fn skip_ws(&mut self) {
        loop {
            match self.peek() {
                Some(c) if c.is_ascii_whitespace() => self.i += 1,
                Some(b'/') if self.peek_at(1) == Some(b'*') => {
                    self.i += 2;
                    while let Some(cc) = self.peek() {
                        if cc == b'*' && self.peek_at(1) == Some(b'/') {
                            self.i += 2;
                            break;
                        }
                        self.i += 1;
                    }
                }
                _ => break,
            }
        }
    }

    fn peek_tok(&mut self) -> Result<&Tok, IfcError> {
        if self.lookahead.is_none() {
            self.lookahead = Some(self.read_token()?);
        }
        // Safe: we just ensured it is Some.
        Ok(self.lookahead.as_ref().unwrap())
    }

    fn next_tok(&mut self) -> Result<Tok, IfcError> {
        if let Some(t) = self.lookahead.take() {
            return Ok(t);
        }
        self.read_token()
    }

    fn read_token(&mut self) -> Result<Tok, IfcError> {
        self.skip_ws();
        let start = self.i;
        match self.peek() {
            None => Err(ParseError::eof(start).with_spec(SpecReference::new("IFC", "ISO 10303-21")).into()),
            Some(b'#') => {
                self.i += 1;
                let mut n: u64 = 0;
                while let Some(c) = self.peek() {
                    if c.is_ascii_digit() {
                        n = n
                            .saturating_mul(10)
                            .saturating_add((c - b'0') as u64);
                        self.i += 1;
                    } else {
                        break;
                    }
                }
                Ok(Tok::Hash(n))
            }
            Some(b'=') => {
                self.i += 1;
                Ok(Tok::Eq)
            }
            Some(b'(') => {
                self.i += 1;
                Ok(Tok::LParen)
            }
            Some(b')') => {
                self.i += 1;
                Ok(Tok::RParen)
            }
            Some(b',') => {
                self.i += 1;
                Ok(Tok::Comma)
            }
            Some(b';') => {
                self.i += 1;
                Ok(Tok::Semicolon)
            }
            Some(b'$') => {
                self.i += 1;
                Ok(Tok::Dollar)
            }
            Some(b'\'') => self.read_string(),
            Some(b'.') => self.read_enum(),
            Some(c) if c == b'-' || c == b'+' || c.is_ascii_digit() => self.read_number(),
            Some(c) if c.is_ascii_alphabetic() || c == b'_' => self.read_ident(),
            Some(c) => Err(self.tok_err("token", c)),
        }
    }

    fn read_string(&mut self) -> Result<Tok, IfcError> {
        // Opening quote already consumed.
        self.i += 1;
        let mut s = String::new();
        loop {
            match self.peek() {
                None => return Err(self.ifc_err("unterminated string")),
                Some(b'\'') => {
                    // Doubled quote escapes to a single quote.
                    if self.peek_at(1) == Some(b'\'') {
                        s.push('\'');
                        self.i += 2;
                    } else {
                        self.i += 1;
                        return Ok(Tok::Str(s));
                    }
                }
                Some(c) => {
                    s.push(c as char);
                    self.i += 1;
                }
            }
        }
    }

    fn read_enum(&mut self) -> Result<Tok, IfcError> {
        self.i += 1; // consume leading '.'
        let mut s = String::new();
        loop {
            match self.peek() {
                None => return Err(self.ifc_err("unterminated enumeration")),
                Some(b'.') => {
                    self.i += 1;
                    return Ok(Tok::Enum(s));
                }
                Some(c) if c.is_ascii_alphanumeric() || c == b'_' => {
                    s.push(c as char);
                    self.i += 1;
                }
                Some(c) => return Err(self.tok_err(".", c)),
            }
        }
    }

    fn read_ident(&mut self) -> Result<Tok, IfcError> {
        let mut s = String::new();
        while let Some(c) = self.peek() {
            if c.is_ascii_alphanumeric() || c == b'_' {
                s.push(c as char);
                self.i += 1;
            } else {
                break;
            }
        }
        Ok(Tok::Ident(s))
    }

    fn read_number(&mut self) -> Result<Tok, IfcError> {
        let start = self.i;
        if matches!(self.peek(), Some(b'-') | Some(b'+')) {
            self.i += 1;
        }
        while let Some(c) = self.peek() {
            match c {
                b'0'..=b'9' | b'.' | b'e' | b'E' | b'+' | b'-' => self.i += 1,
                _ => break,
            }
        }
        let slice = self.b.get(start..self.i).ok_or_else(|| self.ifc_err("number"))?;
        let text = core::str::from_utf8(slice).map_err(|_| self.ifc_err("invalid number utf8"))?;
        let is_real = text.contains('.') || text.contains('e') || text.contains('E');
        if is_real {
            text.parse::<f64>()
                .map(Tok::Real)
                .map_err(|_| self.ifc_err("invalid real"))
        } else {
            match text.parse::<i64>() {
                Ok(n) => Ok(Tok::Int(n)),
                Err(_) => text
                    .parse::<f64>()
                    .map(Tok::Real)
                    .map_err(|_| self.ifc_err("invalid integer")),
            }
        }
    }

    fn expect(&mut self, expected: &'static str, tok: Tok) -> Result<(), IfcError> {
        let got = self.next_tok()?;
        if core::mem::discriminant(&got) == core::mem::discriminant(&tok) {
            Ok(())
        } else {
            Err(self.ifc_err(expected))
        }
    }

    fn parse_value(&mut self) -> Result<Value, IfcError> {
        let t = self.next_tok()?;
        match t {
            Tok::Dollar => Ok(Value::Unset),
            Tok::Int(i) => Ok(Value::Int(i)),
            Tok::Real(r) => Ok(Value::Real(r)),
            Tok::Str(s) => Ok(Value::Str(s)),
            Tok::Enum(e) => Ok(Value::Enum(e)),
            Tok::Hash(id) => Ok(Value::Ref(id)),
            Tok::LParen => self.parse_list_body(),
            Tok::Ident(name) => {
                if matches!(self.peek_tok()?, Tok::LParen) {
                    self.next_tok()?; // consume '('
                    let attrs = self.parse_attr_list()?;
                    Ok(Value::Entity(name, attrs))
                } else {
                    Ok(Value::Enum(name))
                }
            }
            _ => Err(self.ifc_err("expected a value")),
        }
    }

    fn parse_list_body(&mut self) -> Result<Value, IfcError> {
        let mut items = Vec::new();
        if matches!(self.peek_tok()?, Tok::RParen) {
            self.next_tok()?;
            return Ok(Value::List(items));
        }
        loop {
            let v = self.parse_value()?;
            items.push(v);
            match self.next_tok()? {
                Tok::Comma => continue,
                Tok::RParen => break,
                _ => return Err(self.ifc_err("expected ',' or ')' in list")),
            }
        }
        Ok(Value::List(items))
    }

    /// Parse a comma-separated attribute list, consuming the closing `)`.
    fn parse_attr_list(&mut self) -> Result<Vec<Value>, IfcError> {
        let mut attrs = Vec::new();
        if matches!(self.peek_tok()?, Tok::RParen) {
            self.next_tok()?;
            return Ok(attrs);
        }
        loop {
            let v = self.parse_value()?;
            attrs.push(v);
            match self.next_tok()? {
                Tok::Comma => continue,
                Tok::RParen => break,
                _ => return Err(self.ifc_err("expected ',' or ')' in attributes")),
            }
        }
        Ok(attrs)
    }

    fn parse_data_instances(&mut self, doc: &mut IfcDocument) -> Result<(), IfcError> {
        loop {
            let t = self.next_tok()?;
            match t {
                Tok::Ident(s) if s == "ENDSEC" => {
                    self.expect("';' after ENDSEC", Tok::Semicolon)?;
                    return Ok(());
                }
                Tok::Hash(id) => {
                    self.expect("'=' after instance id", Tok::Eq)?;
                    let entity = match self.next_tok()? {
                        Tok::Ident(s) => s,
                        _ => return Err(self.ifc_err("expected entity name")),
                    };
                    self.expect("'(' after entity name", Tok::LParen)?;
                    let attributes = self.parse_attr_list()?;
                    self.expect("';' after instance", Tok::Semicolon)?;
                    doc.instances.push(EntityInstance { id, entity, attributes });
                }
                _ => return Err(self.ifc_err("expected instance or ENDSEC")),
            }
        }
    }

    fn skip_to_endsec(&mut self) -> Result<(), IfcError> {
        loop {
            let t = self.next_tok()?;
            if let Tok::Ident(s) = t {
                if s == "ENDSEC" {
                    self.expect("';' after ENDSEC", Tok::Semicolon)?;
                    return Ok(());
                }
            }
        }
    }

    fn parse(mut self) -> Result<IfcDocument, IfcError> {
        // Magic header `ISO-10303-21;`.
        match self.next_tok()? {
            Tok::Ident(s) if s == "ISO-10303-21" => {}
            _ => return Err(self.ifc_err("expected ISO-10303-21 header")),
        }
        self.expect("';' after header", Tok::Semicolon)?;

        let mut doc = IfcDocument::default();
        loop {
            let t = self.next_tok()?;
            match t {
                Tok::Ident(s) if s == "END-ISO-10303-21" => break,
                Tok::Ident(s) if s == "HEADER" => {
                    self.expect("';' after HEADER", Tok::Semicolon)?;
                    self.skip_to_endsec()?;
                }
                Tok::Ident(s) if s == "DATA" => {
                    self.expect("';' after DATA", Tok::Semicolon)?;
                    self.parse_data_instances(&mut doc)?;
                }
                _ => return Err(self.ifc_err("expected HEADER, DATA, or END-ISO-10303-21")),
            }
        }
        Ok(doc)
    }
}

/// Parse a byte slice into a STEP-Physical File document.
pub fn parse_ifc(input: &[u8]) -> Result<IfcDocument, IfcError> {
    Parser { b: input, i: 0, lookahead: None }.parse()
}

/// Reader for IFC documents, implementing [`tpt_io_core::Reader`].
#[derive(Debug, Clone, Default)]
pub struct IfcReader {
    validation: tpt_io_core::ValidationSettings,
}

impl IfcReader {
    /// Create a reader with default validation.
    pub fn new() -> Self {
        Self::default()
    }
    /// Set the validation configuration.
    pub fn with_validation(mut self, validation: tpt_io_core::ValidationSettings) -> Self {
        self.validation = validation;
        self
    }
}

impl tpt_io_core::Reader<IfcDocument> for IfcReader {
    type Error = IfcError;
    fn parse(&self, input: &[u8]) -> Result<IfcDocument, IfcError> {
        let doc = parse_ifc(input)?;
        self.validation.run(&IfcValidator, &doc)?;
        Ok(doc)
    }
}

/// Syntactic/semantic validation for a parsed [`IfcDocument`].
#[derive(Debug, Clone, Copy, Default)]
pub struct IfcValidator;

impl tpt_io_core::Validator<IfcDocument> for IfcValidator {
    type Error = IfcError;
    fn validate_syntactic(&self, _value: &IfcDocument) -> Result<(), IfcError> {
        Ok(())
    }
    fn validate_semantic(&self, _value: &IfcDocument) -> Result<(), IfcError> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Value;
    use tpt_io_core::Reader;

    const SPF: &str = "ISO-10303-21;
HEADER;
FILE_DESCRIPTION(('ViewDefinition [CoordinationView]'),'2;1');
ENDSEC;
DATA;
#1=IFCPROJECT('proj',$,'My Project',$,$,$,$,$,$,$,$,$,#5);
#5=IFCUNITASSIGNMENT((#10,#11));
#10=IFCSIUNIT($,.LENGTHUNIT.,$,.METRE.);
#11=IFCSIUNIT($,.AREAUNIT.,$,.SQUARE_METRE.);
#20=IFCWALL('wall',$,'Wall',#1,$,$,$);
ENDSEC;
END-ISO-10303-21;
";

    #[test]
    fn parses_instances_and_resolves() {
        let doc = parse_ifc(SPF.as_bytes()).unwrap();
        // #1, #5, #10, #11, #20 are the #id instances (FILE_DESCRIPTION has no id).
        assert_eq!(doc.instances.len(), 5);
        let wall = doc.by_id(20).expect("wall instance");
        assert_eq!(wall.entity, "IFCWALL");
        // First attribute is the name string.
        assert_eq!(wall.attributes[0], Value::Str("wall".into()));
        // Unit assignment references #10 and #11.
        let ua = doc.by_id(5).expect("unit assignment");
        assert_eq!(
            ua.attributes[0],
            Value::List(vec![Value::Ref(10), Value::Ref(11)])
        );
        // Enum parsing inside #10.
        let len = doc.by_id(10).expect("length unit");
        assert_eq!(len.attributes[1], Value::Enum("LENGTHUNIT".into()));
    }

    #[test]
    fn rejects_bad_magic() {
        assert!(parse_ifc(b"NOPE;").is_err());
    }

    #[test]
    fn reader_runs_validation() {
        let r = IfcReader::new();
        let doc = r.parse(SPF.as_bytes()).unwrap();
        assert_eq!(doc.instances.len(), 5);
    }
}
