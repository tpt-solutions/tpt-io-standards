//! S-expression tokenizer for LISP-style netlist and schematic formats.
//!
//! Formats such as EDIF are balanced-parenthesis documents. Tokens are `(`,
//! `)`, and atoms. An atom is either:
//! - a *symbol*: an identifier such as `edif`, `library` or `IN`;
//! - a *number*: an integer (`42`, `-7`) or a real (`3.14`, `1e-3`);
//! - a *string*: double-quoted, with backslash escapes.
//!
//! `;` begins a comment that runs to the end of the line. All tokens own their
//! text, so a parser can build a tree from them without borrowing the input.

use alloc::string::{String, ToString};
use alloc::vec::Vec;

use crate::error::{ParseError, Result};

/// A single lexical token.
#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    /// `(`
    LParen,
    /// `)`
    RParen,
    /// An identifier or keyword such as `edif` or `IN`.
    Symbol(String),
    /// An integer or real number.
    Number(f64),
    /// A double-quoted string literal, with escapes resolved.
    Str(String),
}

impl Token {
    /// Human-readable name of the token kind, for error messages.
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

fn is_space(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\r' | b'\n' | 0x0c)
}

/// Tokenize an S-expression byte slice.
pub fn tokenize(input: &[u8]) -> Result<Vec<Located>> {
    let mut tokens = Vec::new();
    let bytes = input;
    let len = bytes.len();
    let mut i = 0usize;

    while i < len {
        let c = bytes[i];

        if is_space(c) {
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
            let mut closed = false;
            while i < len {
                let b = bytes[i];
                if b == b'"' {
                    i += 1;
                    closed = true;
                    break;
                }
                if b == b'\\' && i + 1 < len {
                    i += 1;
                    s.push(match bytes[i] {
                        b'n' => '\n',
                        b't' => '\t',
                        b'r' => '\r',
                        other => other as char,
                    });
                    i += 1;
                    continue;
                }
                // Strings are byte-oriented; map each byte to a char.
                s.push(b as char);
                i += 1;
            }
            if !closed {
                return Err(ParseError::eof(len));
            }
            tokens.push(Located { tok: Token::Str(s), offset: start });
            continue;
        }

        // Symbol or number: runs until a delimiter.
        let start = i;
        while i < len {
            let b = bytes[i];
            if matches!(b, b'(' | b')' | b'"' | b';') || is_space(b) {
                break;
            }
            i += 1;
        }
        let text = String::from_utf8_lossy(&bytes[start..i]).into_owned();
        match parse_number(&text) {
            Some(n) => tokens.push(Located { tok: Token::Number(n), offset: start }),
            None => tokens.push(Located { tok: Token::Symbol(text.to_string()), offset: start }),
        }
    }

    Ok(tokens)
}

/// Parse an integer or real (case-insensitive exponent, optional sign).
fn parse_number(s: &str) -> Option<f64> {
    let t = s.trim();
    if t.is_empty() {
        return None;
    }
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
    fn numbers_strings_and_comments() {
        let toks = tokenize(b"; comment\n(edifVersion 2 0 0) \"hi\\nthere\"").unwrap();
        assert_eq!(toks.len(), 7);
        assert_eq!(toks[1].tok, Token::Symbol("edifVersion".into()));
        assert_eq!(toks[2].tok, Token::Number(2.0));
        assert_eq!(toks[6].tok, Token::Str("hi\nthere".into()));
    }

    #[test]
    fn negative_and_exponent_numbers() {
        let toks = tokenize(b"-7 1E-3").unwrap();
        assert_eq!(toks[0].tok, Token::Number(-7.0));
        assert_eq!(toks[1].tok, Token::Number(1e-3));
    }

    #[test]
    fn offsets_and_unterminated_string() {
        let toks = tokenize(b"  (a)").unwrap();
        assert_eq!(toks[0].offset, 2);
        assert_eq!(toks[1].offset, 3);
        assert!(tokenize(b"\"open").is_err());
    }
}
