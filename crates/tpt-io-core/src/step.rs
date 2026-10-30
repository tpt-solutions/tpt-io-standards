//! STEP Part 21 (ISO 10303-21, "STEP-Physical File") tokenizer.
//!
//! The lexical grammar shared by every STEP exchange file (IFC, STEP AP203/214/242):
//! identifiers and keywords (`HEADER`, `ISO-10303-21`, `IFCWALL`), entity
//! references (`#12`), enumerations (`.RED.`), string literals (`'…'`, with `''`
//! as an escaped quote), integers, reals, the delimiters `( ) , ; =`, the derived
//! marker `$` and the re-derived marker `*`. Line comments (`//`) and nested
//! block comments (`(* … *)`) are skipped.
//!
//! The tokenizer is zero-copy in its input and records the byte offset of each
//! token, so callers can report precise errors. It does not interpret keywords;
//! a format crate maps [`StepKind::Ident`] to the section keywords it cares about.

use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

use crate::error::ParseError;

/// A STEP token paired with its byte offset in the input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StepToken {
    /// Byte offset of the token's first character.
    pub offset: usize,
    /// The token kind and, where applicable, its text.
    pub kind: StepKind,
}

/// The syntactic category of a STEP token.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StepKind {
    /// An identifier or keyword. Hyphens are included, so `ISO-10303-21` is one token.
    Ident(String),
    /// `;`
    Semi,
    /// `=`
    Equals,
    /// `(`
    LParen,
    /// `)`
    RParen,
    /// `,`
    Comma,
    /// `$`, an unset or derived value.
    Derived,
    /// `*`, a redefined value (equivalent to `$` in most readers).
    Redefined,
    /// `#n`, an entity instance reference.
    Ref(usize),
    /// `.NAME.`, an enumeration literal (text without the dots).
    Enum(String),
    /// `'…'`, a string literal (quotes removed, `''` unescaped).
    String(String),
    /// An integer literal, as written.
    Integer(String),
    /// A real literal, as written.
    Real(String),
}

fn err(offset: usize, reason: impl ToString) -> ParseError {
    ParseError::custom(offset, reason)
}

/// Tokenize a complete STEP exchange file.
///
/// Returns every token, including the `ISO-10303-21` header and the
/// `END-ISO-10303-21` trailer, so that the caller can validate the structure.
pub fn tokenize(input: &str) -> Result<Vec<StepToken>, ParseError> {
    let bytes = input.as_bytes();
    let mut tokens: Vec<StepToken> = Vec::new();
    let mut i = 0usize;
    let len = bytes.len();

    while i < len {
        let c = bytes[i];

        if c.is_ascii_whitespace() {
            i += 1;
            continue;
        }

        // Line comment: // … to end of line.
        if c == b'/' && i + 1 < len && bytes[i + 1] == b'/' {
            i += 2;
            while i < len && bytes[i] != b'\n' {
                i += 1;
            }
            continue;
        }

        // Block comment `(* … *)`, which may nest. `(*` is also an open paren
        // followed by the redefined marker, as in `IFCSIUNIT(*,.LENGTHUNIT.,…)`.
        // It is a comment only when the `*` is not followed by `,` or `)`.
        if c == b'(' && i + 1 < len && bytes[i + 1] == b'*' {
            let after_star = if i + 2 < len { bytes[i + 2] } else { 0 };
            if after_star != b',' && after_star != b')' {
                i = skip_block_comment(bytes, i)?;
                continue;
            }
        }

        if c == b'\'' {
            let (s, next) = read_string(bytes, i)?;
            tokens.push(StepToken { offset: i, kind: StepKind::String(s) });
            i = next;
            continue;
        }

        let single = match c {
            b';' => Some(StepKind::Semi),
            b'=' => Some(StepKind::Equals),
            b'(' => Some(StepKind::LParen),
            b')' => Some(StepKind::RParen),
            b',' => Some(StepKind::Comma),
            b'$' => Some(StepKind::Derived),
            b'*' => Some(StepKind::Redefined),
            _ => None,
        };
        if let Some(kind) = single {
            tokens.push(StepToken { offset: i, kind });
            i += 1;
            continue;
        }

        // Entity instance reference `#n`.
        if c == b'#' {
            let start = i;
            i += 1;
            let num_start = i;
            while i < len && bytes[i].is_ascii_digit() {
                i += 1;
            }
            if i == num_start {
                return Err(err(start, "entity reference needs digits after '#'"));
            }
            let text = core::str::from_utf8(&bytes[num_start..i]).unwrap_or("");
            let id: usize =
                text.parse().map_err(|_| err(start, format!("invalid entity reference #{text}")))?;
            tokens.push(StepToken { offset: start, kind: StepKind::Ref(id) });
            continue;
        }

        // Enumeration `.NAME.`.
        if c == b'.' && i + 1 < len && bytes[i + 1].is_ascii_alphabetic() {
            let (name, next) = read_enum(bytes, i)?;
            tokens.push(StepToken { offset: i, kind: StepKind::Enum(name) });
            i = next;
            continue;
        }

        if c == b'.' && i + 1 < len && bytes[i + 1] == b'.' {
            return Err(err(i, "unexpected '..'"));
        }

        // Number: optional sign, digits, optional fraction and exponent.
        if c.is_ascii_digit()
            || ((c == b'-' || c == b'+') && i + 1 < len && bytes[i + 1].is_ascii_digit())
        {
            let (text, next, is_real) = read_number(bytes, i)?;
            let kind = if is_real { StepKind::Real(text) } else { StepKind::Integer(text) };
            tokens.push(StepToken { offset: i, kind });
            i = next;
            continue;
        }

        if c.is_ascii_alphabetic() {
            let (ident, next) = read_ident(bytes, i);
            tokens.push(StepToken { offset: i, kind: StepKind::Ident(ident) });
            i = next;
            continue;
        }

        return Err(err(i, format!("unexpected character {:?}", c as char)));
    }

    Ok(tokens)
}

/// Skip a nested `(* … *)` comment that starts at the `(` at `start`.
fn skip_block_comment(bytes: &[u8], start: usize) -> Result<usize, ParseError> {
    let mut i = start + 2;
    let mut depth = 1usize;
    let len = bytes.len();
    while i + 1 < len && depth > 0 {
        if bytes[i] == b'(' && bytes[i + 1] == b'*' {
            depth += 1;
            i += 2;
        } else if bytes[i] == b'*' && bytes[i + 1] == b')' {
            depth -= 1;
            i += 2;
        } else {
            i += 1;
        }
    }
    if depth > 0 {
        return Err(err(start, "unterminated block comment"));
    }
    Ok(i)
}

/// Read a string literal that opens with the `'` at `start`. Returns the
/// unescaped text and the index just past the closing quote.
fn read_string(bytes: &[u8], start: usize) -> Result<(String, usize), ParseError> {
    let mut i = start + 1;
    let len = bytes.len();
    let mut out = String::new();
    while i < len {
        let c = bytes[i];
        if c == b'\'' {
            if i + 1 < len && bytes[i + 1] == b'\'' {
                out.push('\'');
                i += 2;
                continue;
            }
            return Ok((out, i + 1));
        }
        // Control characters other than tab are not allowed inside a string.
        if c < 0x20 && c != b'\t' {
            return Err(err(i, "control character in string literal"));
        }
        out.push(c as char);
        i += 1;
    }
    Err(err(start, "unterminated string literal"))
}

/// Read an enumeration `.NAME.` that opens with the `.` at `start`.
fn read_enum(bytes: &[u8], start: usize) -> Result<(String, usize), ParseError> {
    let mut i = start + 1;
    let len = bytes.len();
    let mut out = String::new();
    while i < len {
        let c = bytes[i];
        if c == b'.' {
            return Ok((out, i + 1));
        }
        if c.is_ascii_alphanumeric() || c == b'_' {
            out.push(c as char);
            i += 1;
        } else {
            return Err(err(i, "invalid character in enumeration"));
        }
    }
    Err(err(start, "unterminated enumeration"))
}

/// Read a number. Returns its text, the index just past it, and whether it is
/// a real (it has a fraction or an exponent).
fn read_number(bytes: &[u8], start: usize) -> Result<(String, usize, bool), ParseError> {
    let mut i = start;
    let len = bytes.len();
    let mut is_real = false;

    if i < len && (bytes[i] == b'+' || bytes[i] == b'-') {
        i += 1;
    }
    let int_start = i;
    while i < len && bytes[i].is_ascii_digit() {
        i += 1;
    }
    if i == int_start {
        return Err(err(start, "number has no digits"));
    }
    if i < len && bytes[i] == b'.' {
        is_real = true;
        i += 1;
        while i < len && bytes[i].is_ascii_digit() {
            i += 1;
        }
    }
    if i < len && (bytes[i] == b'E' || bytes[i] == b'e') {
        is_real = true;
        i += 1;
        if i < len && (bytes[i] == b'+' || bytes[i] == b'-') {
            i += 1;
        }
        let exp_start = i;
        while i < len && bytes[i].is_ascii_digit() {
            i += 1;
        }
        if i == exp_start {
            let text = core::str::from_utf8(&bytes[start..i]).unwrap_or("?");
            return Err(err(start, format!("invalid exponent in {text:?}")));
        }
    }

    let text = core::str::from_utf8(&bytes[start..i]).unwrap_or("").to_string();
    Ok((text, i, is_real))
}

/// Read an identifier. Hyphens are part of identifiers so that `ISO-10303-21`
/// and `END-ISO-10303-21` each come out as one token.
fn read_ident(bytes: &[u8], start: usize) -> (String, usize) {
    let mut i = start;
    let len = bytes.len();
    while i < len && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_' || bytes[i] == b'-') {
        i += 1;
    }
    let text = core::str::from_utf8(&bytes[start..i]).unwrap_or("").to_string();
    (text, i)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(src: &str) -> Vec<StepKind> {
        tokenize(src).unwrap().into_iter().map(|t| t.kind).collect()
    }

    #[test]
    fn tokenizes_header_and_instance() {
        let k = kinds("ISO-10303-21;\n#1 = IFCWALL('abc');\nEND-ISO-10303-21;\n");
        assert_eq!(k[0], StepKind::Ident("ISO-10303-21".into()));
        assert_eq!(k[1], StepKind::Semi);
        assert_eq!(k[2], StepKind::Ref(1));
        assert_eq!(k[3], StepKind::Equals);
        assert_eq!(k[4], StepKind::Ident("IFCWALL".into()));
        assert_eq!(k[5], StepKind::LParen);
        assert_eq!(k[6], StepKind::String("abc".into()));
        assert_eq!(k[7], StepKind::RParen);
        assert_eq!(k[8], StepKind::Semi);
        assert_eq!(k[9], StepKind::Ident("END-ISO-10303-21".into()));
    }

    #[test]
    fn strings_enums_and_numbers() {
        let k = kinds("#2 = IFC('Hello''World',.RED.,42,3.14,1.0E-3,-7);");
        assert_eq!(k[4], StepKind::String("Hello'World".into()));
        assert_eq!(k[6], StepKind::Enum("RED".into()));
        assert_eq!(k[8], StepKind::Integer("42".into()));
        assert_eq!(k[10], StepKind::Real("3.14".into()));
        assert_eq!(k[12], StepKind::Real("1.0E-3".into()));
        assert_eq!(k[14], StepKind::Integer("-7".into()));
    }

    #[test]
    fn derived_and_redefined_markers() {
        let k = kinds("#3 = P((0.,$,*));");
        assert_eq!(k[7], StepKind::Derived);
        assert_eq!(k[9], StepKind::Redefined);
    }

    #[test]
    fn skips_line_and_nested_block_comments() {
        let k = kinds("// note\nA; (* outer (* inner *) still *) B;");
        assert_eq!(k[0], StepKind::Ident("A".into()));
        assert_eq!(k[2], StepKind::Ident("B".into()));
    }

    #[test]
    fn open_paren_star_is_not_a_comment_before_comma() {
        // `(*,` is an open paren followed by the redefined marker.
        let k = kinds("F(*,.X.);");
        assert_eq!(k[1], StepKind::LParen);
        assert_eq!(k[2], StepKind::Redefined);
    }

    #[test]
    fn offsets_point_at_token_starts() {
        let toks = tokenize("  #7 = X;").unwrap();
        assert_eq!(toks[0].offset, 2);
        assert_eq!(toks[1].offset, 5);
    }

    #[test]
    fn errors_carry_offsets() {
        assert_eq!(tokenize("'open").unwrap_err().offset, 0);
        assert_eq!(tokenize("(* never ends").unwrap_err().offset, 0);
        assert_eq!(tokenize("A #x").unwrap_err().offset, 2);
        assert!(tokenize("#").is_err());
        assert!(tokenize("1.0E+").is_err());
    }
}
