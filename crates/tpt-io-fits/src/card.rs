//! FITS header cards: keyword, typed value and comment, in the fixed 80-column layout.

use alloc::string::{String, ToString};

use crate::error::FitsError;

/// The value of a card.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    /// `T` or `F`.
    Logical(bool),
    /// An integer.
    Integer(i64),
    /// A real number (`E` or `D` exponent accepted on input).
    Real(f64),
    /// A quoted string, with doubled quotes resolved and trailing spaces removed.
    Text(String),
}

/// One 80-character header card.
#[derive(Debug, Clone, PartialEq)]
pub struct Card {
    /// The keyword, up to 8 characters, e.g. `BITPIX`. Commentary cards use `COMMENT`, `HISTORY`
    /// or an empty keyword.
    pub keyword: String,
    /// The value, or `None` for commentary cards and `END`.
    pub value: Option<Value>,
    /// The comment after the `/`, if any.
    pub comment: Option<String>,
}

impl Card {
    /// A card with a keyword and value and no comment.
    pub fn new(keyword: &str, value: Value) -> Self {
        Card { keyword: keyword.to_string(), value: Some(value), comment: None }
    }

    /// A commentary card (`COMMENT`, `HISTORY` or blank keyword) holding free text.
    pub fn commentary(keyword: &str, text: &str) -> Self {
        Card { keyword: keyword.to_string(), value: None, comment: Some(text.to_string()) }
    }

    /// The card's integer value, if it has one.
    pub fn as_integer(&self) -> Option<i64> {
        match self.value {
            Some(Value::Integer(i)) => Some(i),
            _ => None,
        }
    }

    /// Parse one 80-byte card image.
    pub fn parse(image: &[u8]) -> Result<Card, FitsError> {
        if image.len() != 80 || !image.is_ascii() {
            return Err(FitsError::Invalid("card must be 80 ASCII bytes".into()));
        }
        let text = core::str::from_utf8(image)
            .map_err(|_| FitsError::Invalid("card is not ASCII".into()))?;
        let keyword = text[..8].trim_end().to_string();
        let has_value = text.as_bytes()[8] == b'=' && text.as_bytes()[9] == b' ';
        if keyword == "END" || !has_value {
            let body = if keyword.is_empty() || keyword == "COMMENT" || keyword == "HISTORY" {
                text[8..].trim_end().to_string()
            } else {
                String::new()
            };
            return Ok(Card {
                keyword,
                value: None,
                comment: if body.is_empty() { None } else { Some(body) },
            });
        }
        let field = &text[10..];
        let (value, comment) = split_value(field)?;
        Ok(Card { keyword, value: Some(value), comment })
    }

    /// Serialise to an 80-byte card image.
    pub fn to_image(&self) -> [u8; 80] {
        let mut s = alloc::string::String::new();
        s.push_str(&alloc::format!("{:<8}", self.keyword));
        match &self.value {
            None => {
                if let Some(c) = &self.comment {
                    if self.keyword == "COMMENT"
                        || self.keyword == "HISTORY"
                        || self.keyword.is_empty()
                    {
                        s.push_str(c);
                    }
                }
            }
            Some(v) => {
                s.push_str("= ");
                let body = match v {
                    Value::Logical(b) => alloc::format!("{:>20}", if *b { "T" } else { "F" }),
                    Value::Integer(i) => alloc::format!("{i:>20}"),
                    Value::Real(r) => alloc::format!("{:>20}", alloc::format!("{r:E}")),
                    Value::Text(t) => {
                        let quoted = alloc::format!("'{}'", t.replace('\'', "''"));
                        alloc::format!("{quoted:<20}")
                    }
                };
                s.push_str(&body);
                if let Some(c) = &self.comment {
                    s.push_str(&alloc::format!(" / {c}"));
                }
            }
        }
        let mut out = [b' '; 80];
        let bytes = s.as_bytes();
        let n = bytes.len().min(80);
        out[..n].copy_from_slice(&bytes[..n]);
        out
    }
}

/// Split the value field (columns 11–80) into its value and optional comment.
fn split_value(field: &str) -> Result<(Value, Option<String>), FitsError> {
    let field = field.trim_start();
    if let Some(rest) = field.strip_prefix('\'') {
        // String: runs to the closing quote; a doubled quote is an escaped quote.
        let mut out = String::new();
        let mut chars = rest.char_indices().peekable();
        let mut end = None;
        while let Some((i, c)) = chars.next() {
            if c == '\'' {
                if matches!(chars.peek(), Some((_, '\''))) {
                    out.push('\'');
                    chars.next();
                } else {
                    end = Some(i + 1);
                    break;
                }
            } else {
                out.push(c);
            }
        }
        let after = match end {
            Some(e) => &rest[e..],
            None => return Err(FitsError::Invalid("unterminated string value".into())),
        };
        let comment =
            after.find('/').map(|i| after[i + 1..].trim().to_string()).filter(|c| !c.is_empty());
        return Ok((Value::Text(out.trim_end().to_string()), comment));
    }
    let (token, comment) = match field.find('/') {
        Some(i) => {
            (field[..i].trim(), Some(field[i + 1..].trim().to_string()).filter(|c| !c.is_empty()))
        }
        None => (field.trim(), None),
    };
    let value = match token {
        "T" => Value::Logical(true),
        "F" => Value::Logical(false),
        "" => return Err(FitsError::Invalid("empty value".into())),
        t if t.contains(['.', 'E', 'e', 'D', 'd']) => {
            let norm = t.replace(['D', 'd'], "E");
            Value::Real(
                norm.parse::<f64>().map_err(|_| FitsError::Invalid("bad real value".into()))?,
            )
        }
        t => Value::Integer(
            t.parse::<i64>().map_err(|_| FitsError::Invalid("bad integer value".into()))?,
        ),
    };
    Ok((value, comment))
}
