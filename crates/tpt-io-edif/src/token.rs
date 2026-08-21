//! EDIF S-expression tokenizer.
//!
//! EDIF is a balanced-parenthesis, LISP-like format (per EDIF 2 0 0 / 4 0 0).
//! Tokens are `(`, `)`, and atoms. Atoms are either:
//! - *symbols* — identifiers such as `edif`, `library`, `IN`;
//! - *numbers* — integers (`42`, `-7`) or reals (`3.14`, `1e-3`);
//! - *strings* — double-quoted, with C-style escapes.
//!
//! `;` begins a comment that runs to end of line. All tokens own their text so
//! the parser can be `no_std` (with `alloc`) and location-preserving.

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use tpt_io_core::{ParseError, Result, SpecReference};

/// A single lexical token.
#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    /// `(`
    LParen,
    /// `)`
    RParen,
    /// An identifier / keyword such as `edif` or `IN`.
    Symbol(String),
    /// An integer or real number.
    Number(f64),
    /// A double-quoted string literal.
    Str(String),
}

impl Token {
    /// Human-readable name, for error messages.
    pub fn kind_name(&self) -> &'static str {
        match self {
            Token::LParen => "(",
            Token::RParen => ")",
            Token::Symbol(_) => "symbol",
            Token::Number(_) => "number",
            Token::Str(_) => "string",
        }
    }
}

/// A token with its byte offset in the original input.
#[derive(Debug, Clone, PartialEq)]
pub struct Located {
    /// The token.
    pub tok: Token,
    /// Byte offset of the token start.
    pub offset: usize,
}

/// Tokenize an EDIF byte slice.
pub fn tokenize(input: &[u8]) -> Result<Vec<Located>> {
    let mut tokens = Vec::new();
    let bytes = input;
    let len = bytes.len();
    let mut i = 0usize;

    while i < len {
        let c = bytes[i];

        // Whitespace.
        if c == b' ' || c == b'\t' || c == b'\r' || c == b'\n' || c == 0x0c {
            i += 1;
            continue;
        }

        // Comment: to end of line.
        if c == b';' {
            while i < len && bytes[i] != b'\n' {
                i += 1;
            }
            continue;
        }

        if c == b'(' {
            tokens.push(Located { tok: Token::LParen, offset: i });
            i += 1;
            continue;
        }
        if c == b')' {
            tokens.push(Located { tok: Token::RParen, offset: i });
            i += 1;
            continue;
        }

        if c == b'"' {
            let start = i;
            i += 1;
            let mut s = String::new();
            while i < len {
                let b = bytes[i];
                if b == b'"' {
                    i += 1;
                    tokens.push(Located { tok: Token::Str(s), offset: start });
                    break;
                }
                if b == b'\\' && i + 1 < len {
                    i += 1;
                    let e = bytes[i];
                    s.push(match e {
                        b'n' => '\n',
                        b't' => '\t',
                        b'r' => '\r',
                        b'"' => '"',
                        b'\\' => '\\',
                        other => other as char,
                    });
                    i += 1;
                    continue;
                }
                // EDIF strings are latin-1-ish; map bytes to chars lossily.
                s.push(b as char);
                i += 1;
            }
            if i > 0 && bytes[i.saturating_sub(1)] != b'"' && i >= len {
                return Err(
                    ParseError::eof(len).with_spec(SpecReference::new("EDIF", "2 0 0 §3.4.1"))
                );
            }
            continue;
        }

        // Symbol / number.
        let start = i;
        while i < len {
            let b = bytes[i];
            if b == b'('
                || b == b')'
                || b == b'"'
                || b == b';'
                || b == b' '
                || b == b'\t'
                || b == b'\r'
                || b == b'\n'
                || b == 0x0c
            {
                break;
            }
            i += 1;
        }
        if i == start {
            // Shouldn't happen given above branches, but guard anyway.
            return Err(ParseError::invalid_byte(start, c)
                .with_spec(SpecReference::new("EDIF", "2 0 0 §3.4")));
        }
        let slice = &bytes[start..i];
        let text = alloc::string::String::from_utf8_lossy(slice).into_owned();
        if let Some(n) = parse_number(&text) {
            tokens.push(Located { tok: Token::Number(n), offset: start });
        } else {
            tokens.push(Located { tok: Token::Symbol(text.to_string()), offset: start });
        }
    }

    Ok(tokens)
}

/// Parse an integer or real from an EDIF number token (case-insensitive exponent).
fn parse_number(s: &str) -> Option<f64> {
    let t = s.trim();
    if t.is_empty() {
        return None;
    }
    // Allow leading + or -.
    let (neg, body) = match t.strip_prefix('+') {
        Some(r) => (false, r),
        None => t.strip_prefix('-').map_or((false, t), |r| (true, r)),
    };
    let is_num = body.chars().enumerate().all(|(idx, ch)| {
        ch.is_ascii_digit()
            || ch == '.'
            || (idx > 0 && (ch == 'e' || ch == 'E' || ch == '+' || ch == '-'))
    });
    if !is_num || body.is_empty() {
        return None;
    }
    let parsed: f64 = body.parse().ok()?;
    Some(if neg { -parsed } else { parsed })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_tokens() {
        let toks = tokenize(b"(edif foo (cell bar))").unwrap();
        assert_eq!(toks.len(), 8);
        assert_eq!(toks[0].tok, Token::LParen);
        assert_eq!(toks[1].tok, Token::Symbol("edif".into()));
        assert_eq!(toks[2].tok, Token::Symbol("foo".into()));
        assert_eq!(toks[6].tok, Token::RParen);
    }

    #[test]
    fn numbers_and_strings_and_comments() {
        let toks = tokenize(b"; comment\n(edifVersion 2 0 0) \"hi\\nthere\"").unwrap();
        // 5 tokens: ( , edifVersion, 2, 0, 0, ) , "hi\nthere"
        assert_eq!(toks.len(), 7);
        assert_eq!(toks[1].tok, Token::Symbol("edifVersion".into()));
        assert_eq!(toks[2].tok, Token::Number(2.0));
        assert_eq!(toks[6].tok, Token::Str("hi\nthere".into()));
    }

    #[test]
    fn negative_number() {
        let toks = tokenize(b"-7").unwrap();
        assert_eq!(toks[0].tok, Token::Number(-7.0));
    }
}
