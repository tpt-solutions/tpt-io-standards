//! STEP-Physical File (SPF / ISO 10303-21) tokenizer.
//!
//! SPF is the ASCII encoding of the IFC data model. Its lexical grammar
//! (ISO 10303-21 §6) consists of: keywords (`ISO-10303-21`, `HEADER`, `DATA`,
//! …), entity references (`#id`), enumerations (`.ENUM.`), string literals
//! (`'…'` with `''` as an escaped quote), integers, reals, delimiters
//! (`(`, `)`, `,`, `;`, `=`), and the derived-marker `$`. Line comments (`//`)
//! and nested block comments (`(* … *)`) are skipped.
//!
//! The tokenizer is zero-copy: every [`Token`] borrows its text from the input
//! and carries a byte offset so the parser can report precise errors.

use alloc::string::{String, ToString};
use alloc::vec::Vec;

use crate::error::IfcError;

/// A SPF token paired with its byte offset in the input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    /// Byte offset of the token's first character.
    pub offset: usize,
    /// The token kind and (where applicable) its text.
    pub kind: TokenKind,
}

/// The syntactic category of a SPF token.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenKind {
    /// `ISO-10303-21` file header keyword.
    IsoHeader,
    /// `END-ISO-10303-21` file trailer keyword.
    IsoTrailer,
    /// `HEADER` section keyword.
    Header,
    /// `DATA` section keyword.
    Data,
    /// `ENDSEC` section terminator.
    EndSec,
    /// `;` statement terminator.
    Semi,
    /// `=` assignment.
    Equals,
    /// `(` aggregate open.
    LParen,
    /// `)` aggregate close.
    RParen,
    /// `,` list separator.
    Comma,
    /// `$` derived / unspecified value marker.
    Derived,
    /// `*` re-derived marker (semantically equivalent to `$`).
    Redefined,
    /// `#id` entity instance reference. The `usize` is the numeric id.
    Ref(usize),
    /// `.ENUM.` enumeration literal.
    Enum(String),
    /// `'…'` string literal (quotes stripped, `''` unescaped).
    String(String),
    /// Integer literal.
    Integer(String),
    /// Real (floating-point) literal.
    Real(String),
    /// An identifier — entity type names (`IFCWALL`) or keyword names.
    Ident(String),
}

/// Tokenize a complete SPF document into a `Vec<Token>`.
///
/// Returns every token (including the mandatory `ISO-10303-21` header and
/// `END-ISO-10303-21` trailer) so the parser can validate document structure.
pub fn tokenize(input: &str) -> Result<Vec<Token>, IfcError> {
    let bytes = input.as_bytes();
    let mut tokens: Vec<Token> = Vec::new();
    let mut i = 0usize;
    let len = bytes.len();

    while i < len {
        let c = bytes[i];

        // Whitespace.
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

        // Block comment: (* … *) — may nest.
        //
        // Ambiguity: `(*` is also an open-paren followed by the redefined
        // marker `*` (e.g. `IFCSIUNIT(*,.LENGTHUNIT.,...)`). We resolve it by
        // treating `(*` as a comment only when the `*` is NOT immediately
        // followed by `,` or `)` (which would make it a standalone value).
        if c == b'(' && i + 1 < len && bytes[i + 1] == b'*' {
            let after_star = if i + 2 < len { bytes[i + 2] } else { 0 };
            if after_star != b',' && after_star != b')' {
                i = skip_block_comment(bytes, i)?;
                continue;
            }
        }

        // String literal.
        if c == b'\'' {
            let (s, next) = read_string(bytes, i)?;
            tokens.push(Token { offset: i, kind: TokenKind::String(s) });
            i = next;
            continue;
        }

        // Single-character delimiters.
        match c {
            b';' => {
                tokens.push(Token { offset: i, kind: TokenKind::Semi });
                i += 1;
                continue;
            }
            b'=' => {
                tokens.push(Token { offset: i, kind: TokenKind::Equals });
                i += 1;
                continue;
            }
            b'(' => {
                tokens.push(Token { offset: i, kind: TokenKind::LParen });
                i += 1;
                continue;
            }
            b')' => {
                tokens.push(Token { offset: i, kind: TokenKind::RParen });
                i += 1;
                continue;
            }
            b',' => {
                tokens.push(Token { offset: i, kind: TokenKind::Comma });
                i += 1;
                continue;
            }
            b'$' => {
                tokens.push(Token { offset: i, kind: TokenKind::Derived });
                i += 1;
                continue;
            }
            b'*' => {
                tokens.push(Token { offset: i, kind: TokenKind::Redefined });
                i += 1;
                continue;
            }
            _ => {}
        }

        // Instance reference #id.
        if c == b'#' {
            let start = i;
            i += 1;
            let num_start = i;
            while i < len && bytes[i].is_ascii_digit() {
                i += 1;
            }
            if i == num_start {
                return Err(IfcError::invalid_instance_id(start, "#"));
            }
            let id: usize = core::str::from_utf8(&bytes[num_start..i])
                .unwrap()
                .parse()
                .map_err(|_| IfcError::invalid_instance_id(start, &input[start..i]))?;
            tokens.push(Token { offset: start, kind: TokenKind::Ref(id) });
            continue;
        }

        // Enumeration .NAME. (leading dot, uppercase letters/digits/underscore).
        if c == b'.' && i + 1 < len && bytes[i + 1].is_ascii_alphabetic() {
            let (enm, next) = read_enum(bytes, i)?;
            tokens.push(Token { offset: i, kind: TokenKind::Enum(enm) });
            i = next;
            continue;
        }

        // Trailing-dot enumeration (e.g. `.` is not valid, but a lone `.X.` is).
        if c == b'.' && i + 1 < len && bytes[i + 1] == b'.' {
            return Err(IfcError::invalid_token(i, ".."));
        }

        // Number: integer or real (optional sign, digits, optional fraction, optional exponent).
        if c.is_ascii_digit()
            || (c == b'-' && i + 1 < len && bytes[i + 1].is_ascii_digit())
            || (c == b'+' && i + 1 < len && bytes[i + 1].is_ascii_digit())
        {
            let (num, next, is_real) = read_number(bytes, i)?;
            let kind = if is_real { TokenKind::Real(num) } else { TokenKind::Integer(num) };
            tokens.push(Token { offset: i, kind });
            i = next;
            continue;
        }

        // Identifier / keyword (uppercase letters, digits, underscore; must start with a letter).
        if c.is_ascii_alphabetic() {
            let (ident, next) = read_ident(bytes, i)?;
            let offset = i;
            let kind = match ident.as_str() {
                "ISO-10303-21" => TokenKind::IsoHeader,
                "END-ISO-10303-21" => TokenKind::IsoTrailer,
                "HEADER" => TokenKind::Header,
                "DATA" => TokenKind::Data,
                "ENDSEC" => TokenKind::EndSec,
                _ => TokenKind::Ident(ident),
            };
            tokens.push(Token { offset, kind });
            i = next;
            continue;
        }

        return Err(IfcError::invalid_token(i, core::str::from_utf8(&[c]).unwrap_or("?")));
    }

    Ok(tokens)
}

/// Skip a nested `(* … *)` block comment starting at the opening `(`.
fn skip_block_comment(bytes: &[u8], start: usize) -> Result<usize, IfcError> {
    let mut i = start + 2; // past `(*`
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
        return Err(IfcError::invalid_token(start, "(*"));
    }
    Ok(i)
}

/// Read a string literal starting at the opening `'`. Returns the unescaped
/// content and the index just past the closing `'`.
fn read_string(bytes: &[u8], start: usize) -> Result<(String, usize), IfcError> {
    let mut i = start + 1; // past opening quote
    let len = bytes.len();
    let mut out = String::new();
    while i < len {
        let c = bytes[i];
        if c == b'\'' {
            if i + 1 < len && bytes[i + 1] == b'\'' {
                // Escaped quote '' → single quote.
                out.push('\'');
                i += 2;
                continue;
            }
            // End of string.
            return Ok((out, i + 1));
        }
        // Per SPF, control characters other than tab are not allowed in strings.
        if c < 0x20 && c != b'\t' {
            return Err(IfcError::invalid_token(i, core::str::from_utf8(&[c]).unwrap_or("?")));
        }
        out.push(c as char);
        i += 1;
    }
    Err(IfcError::unterminated_string(start))
}

/// Read an enumeration literal `.NAME.` starting at the leading dot.
fn read_enum(bytes: &[u8], start: usize) -> Result<(String, usize), IfcError> {
    let mut i = start + 1; // past leading dot
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
            return Err(IfcError::invalid_token(i, core::str::from_utf8(&[c]).unwrap_or("?")));
        }
    }
    Err(IfcError::invalid_token(start, "."))
}

/// Read a numeric literal. Returns the raw text, the index past it, and whether
/// it contains a fractional part or exponent (i.e. is a real).
fn read_number(bytes: &[u8], start: usize) -> Result<(String, usize, bool), IfcError> {
    let mut i = start;
    let len = bytes.len();
    let mut is_real = false;

    // Optional sign.
    if i < len && (bytes[i] == b'+' || bytes[i] == b'-') {
        i += 1;
    }
    // Integer part.
    let int_start = i;
    while i < len && bytes[i].is_ascii_digit() {
        i += 1;
    }
    if i == int_start {
        return Err(IfcError::invalid_number(start, ""));
    }
    // Fractional part.
    if i < len && bytes[i] == b'.' {
        is_real = true;
        i += 1;
        while i < len && bytes[i].is_ascii_digit() {
            i += 1;
        }
    }
    // Exponent.
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
            return Err(IfcError::invalid_number(
                start,
                core::str::from_utf8(&bytes[start..i]).unwrap_or("?"),
            ));
        }
    }

    let text = core::str::from_utf8(&bytes[start..i]).unwrap_or("").to_string();
    Ok((text, i, is_real))
}

/// Read an identifier (entity type name or keyword). Hyphens are included so
/// that the SPF keywords `ISO-10303-21` and `END-ISO-10303-21` are read as a
/// single token.
fn read_ident(bytes: &[u8], start: usize) -> Result<(String, usize), IfcError> {
    let mut i = start;
    let len = bytes.len();
    while i < len && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_' || bytes[i] == b'-') {
        i += 1;
    }
    let text = core::str::from_utf8(&bytes[start..i]).unwrap_or("").to_string();
    Ok((text, i))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokenizes_simple_instance() {
        let toks = tokenize("ISO-10303-21;\nHEADER;\nENDSEC;\nDATA;\n#1 = IFCWALL('abc');\nENDSEC;\nEND-ISO-10303-21;\n").unwrap();
        assert!(matches!(&toks[0].kind, TokenKind::IsoHeader));
        assert!(matches!(&toks[1].kind, TokenKind::Semi));
        assert!(matches!(&toks[2].kind, TokenKind::Header));
        assert!(matches!(&toks[3].kind, TokenKind::Semi));
        assert!(matches!(&toks[4].kind, TokenKind::EndSec));
        assert!(matches!(&toks[5].kind, TokenKind::Semi));
        assert!(matches!(&toks[6].kind, TokenKind::Data));
        assert!(matches!(&toks[7].kind, TokenKind::Semi));
        assert!(matches!(&toks[8].kind, TokenKind::Ref(1)));
        assert!(matches!(&toks[9].kind, TokenKind::Equals));
        if let TokenKind::Ident(n) = &toks[10].kind {
            assert_eq!(n, "IFCWALL");
        } else {
            panic!();
        }
        assert!(matches!(&toks[11].kind, TokenKind::LParen));
        if let TokenKind::String(s) = &toks[12].kind {
            assert_eq!(s, "abc");
        } else {
            panic!();
        }
        assert!(matches!(&toks[13].kind, TokenKind::RParen));
        assert!(matches!(&toks[14].kind, TokenKind::Semi));
        assert!(matches!(&toks[15].kind, TokenKind::EndSec));
        assert!(matches!(&toks[16].kind, TokenKind::Semi));
        assert!(matches!(&toks[17].kind, TokenKind::IsoTrailer));
        assert!(matches!(&toks[18].kind, TokenKind::Semi));
    }

    #[test]
    fn handles_strings_enums_numbers() {
        let src = "#2 = IFC('Hello''World',.RED.,42,3.14,1.0E-3);";
        let toks = tokenize(src).unwrap();
        assert!(matches!(&toks[0].kind, TokenKind::Ref(2)));
        if let TokenKind::String(s) = &toks[4].kind {
            assert_eq!(s, "Hello'World");
        } else {
            panic!();
        }
        if let TokenKind::Enum(e) = &toks[6].kind {
            assert_eq!(e, "RED");
        } else {
            panic!();
        }
        if let TokenKind::Integer(n) = &toks[8].kind {
            assert_eq!(n, "42");
        } else {
            panic!();
        }
        if let TokenKind::Real(r) = &toks[10].kind {
            assert_eq!(r, "3.14");
        } else {
            panic!();
        }
        if let TokenKind::Real(r) = &toks[12].kind {
            assert_eq!(r, "1.0E-3");
        } else {
            panic!();
        }
    }

    #[test]
    fn skips_comments() {
        let src = "// line comment\nISO-10303-21; (* block (* nested *) comment *) HEADER;";
        let toks = tokenize(src).unwrap();
        assert!(matches!(&toks[0].kind, TokenKind::IsoHeader));
        assert!(matches!(&toks[2].kind, TokenKind::Header));
    }

    #[test]
    fn unterminated_string_errors() {
        assert!(tokenize("'unterminated").is_err());
    }

    #[test]
    fn unterminated_block_comment_errors() {
        assert!(tokenize("(* never ends").is_err());
    }

    #[test]
    fn invalid_instance_id_errors() {
        assert!(tokenize("#abc").is_err());
    }

    #[test]
    fn derived_and_redefined() {
        let toks = tokenize("#3 = IFCCARTESIANPOINT((0.,$));").unwrap();
        // Tokens: Ref, Equals, Ident, LParen, LParen, Real, Comma, Derived, RParen, RParen, Semi.
        assert!(matches!(&toks[4].kind, TokenKind::LParen));
        assert!(matches!(&toks[5].kind, TokenKind::Real(_)));
        assert!(matches!(&toks[7].kind, TokenKind::Derived));
    }
}
