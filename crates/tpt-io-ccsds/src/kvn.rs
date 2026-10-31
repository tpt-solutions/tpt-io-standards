//! Shared KVN line handling: `KEY = VALUE` pairs, comments and blank lines.

use alloc::vec::Vec;

use tpt_io_core::ParseError;

use crate::error::CcsdsError;

/// One classified line of a KVN message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Line<'a> {
    /// `KEY = VALUE`, with the key trimmed and the value trimmed.
    Pair(&'a str, &'a str),
    /// A bare keyword such as `META_START`, `META_STOP` or `COVARIANCE_START`.
    Keyword(&'a str),
    /// Anything else: data rows, comments and unknown text.
    Other(&'a str),
}

/// Classify a line. Blank lines and `COMMENT` lines return `None`.
pub fn classify(line: &str) -> Option<Line<'_>> {
    let t = line.trim();
    if t.is_empty() || t.starts_with("COMMENT") {
        return None;
    }
    if let Some((k, v)) = t.split_once('=') {
        let k = k.trim();
        if !k.is_empty() && k.bytes().all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'_') {
            return Some(Line::Pair(k, v.trim()));
        }
    }
    if t.bytes().all(|b| b.is_ascii_uppercase() || b == b'_') {
        return Some(Line::Keyword(t));
    }
    Some(Line::Other(t))
}

/// Split the text into lines (accepting `\n` or `\r\n`).
pub fn lines(input: &[u8]) -> Result<Vec<&str>, CcsdsError> {
    core::str::from_utf8(input)
        .map(|s| s.lines().collect())
        .map_err(|e| ParseError::custom(e.valid_up_to(), "message is not valid UTF-8").into())
}

/// A number from a KVN value or a data column.
pub fn number(s: &str, what: &str) -> Result<f64, CcsdsError> {
    s.trim()
        .parse::<f64>()
        .map_err(|_| CcsdsError::Invalid(alloc::format!("{what} is not a number")))
}
