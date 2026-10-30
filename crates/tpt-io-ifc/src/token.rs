//! STEP-Physical File (SPF / ISO 10303-21) tokenizer.
//!
//! SPF is the ASCII encoding of the IFC data model. Its lexical grammar
//! (ISO 10303-21 §6) consists of: keywords (`ISO-10303-21`, `HEADER`, `DATA`,
//! …), entity references (`#id`), enumerations (`.ENUM.`), string literals
//! (`'…'` with `''` as an escaped quote), integers, reals, delimiters
//! (`(`, `)`, `,`, `;`, `=`), and the derived-marker `$`. Line comments (`//`)
//! and nested block comments (`(* … *)`) are skipped.
//!
//! The lexing itself lives in [`tpt_io_core::step`], shared with the other STEP
//! exchange formats. This module maps its tokens onto IFC's section keywords.
//! Every [`Token`] carries a byte offset so the parser can report precise errors.

use alloc::string::String;
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
/// The lexical work is done by [`tpt_io_core::step`]; this maps its neutral
/// tokens onto IFC's section keywords. Returns every token, including the
/// mandatory `ISO-10303-21` header and `END-ISO-10303-21` trailer, so the parser
/// can validate document structure.
pub fn tokenize(input: &str) -> Result<Vec<Token>, IfcError> {
    use tpt_io_core::step::{tokenize as step_tokenize, StepKind};

    let tokens = step_tokenize(input)?;
    Ok(tokens
        .into_iter()
        .map(|t| {
            let kind = match t.kind {
                StepKind::Ident(ident) => match ident.as_str() {
                    "ISO-10303-21" => TokenKind::IsoHeader,
                    "END-ISO-10303-21" => TokenKind::IsoTrailer,
                    "HEADER" => TokenKind::Header,
                    "DATA" => TokenKind::Data,
                    "ENDSEC" => TokenKind::EndSec,
                    _ => TokenKind::Ident(ident),
                },
                StepKind::Semi => TokenKind::Semi,
                StepKind::Equals => TokenKind::Equals,
                StepKind::LParen => TokenKind::LParen,
                StepKind::RParen => TokenKind::RParen,
                StepKind::Comma => TokenKind::Comma,
                StepKind::Derived => TokenKind::Derived,
                StepKind::Redefined => TokenKind::Redefined,
                StepKind::Ref(id) => TokenKind::Ref(id),
                StepKind::Enum(name) => TokenKind::Enum(name),
                StepKind::String(s) => TokenKind::String(s),
                StepKind::Integer(n) => TokenKind::Integer(n),
                StepKind::Real(r) => TokenKind::Real(r),
            };
            Token { offset: t.offset, kind }
        })
        .collect())
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
