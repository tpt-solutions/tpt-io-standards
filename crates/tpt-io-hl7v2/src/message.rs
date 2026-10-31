//! HL7 v2 message model, parsing and writing.

use alloc::string::{String, ToString};
use alloc::vec::Vec;

use crate::error::Hl7Error;

/// The delimiters declared in MSH-1 and MSH-2.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Separators {
    /// Field separator (MSH-1), normally `|`.
    pub field: char,
    /// Component separator, normally `^`.
    pub component: char,
    /// Repetition separator, normally `~`.
    pub repetition: char,
    /// Escape character, normally `\`.
    pub escape: char,
    /// Subcomponent separator, normally `&`.
    pub subcomponent: char,
}

/// One segment: its three-letter id and its raw field texts (the id is not repeated in `fields`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Segment {
    /// The segment id, e.g. `MSH` or `PID`.
    pub id: String,
    /// Field texts in order. For `MSH`, `fields[0]` is the encoding characters (MSH-2).
    pub fields: Vec<String>,
}

impl Segment {
    /// The HL7 field `n` (1-based), as raw text. For `MSH`, field 1 is the field separator itself.
    pub fn field(&self, n: usize) -> Option<&str> {
        if self.id == "MSH" {
            return match n {
                0 => None,
                1 => Some("|"),
                _ => self.fields.get(n - 2).map(|s| s.as_str()),
            };
        }
        if n == 0 {
            return None;
        }
        self.fields.get(n - 1).map(|s| s.as_str())
    }
}

/// A message: its delimiters and its segments.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Message {
    /// The delimiters from MSH.
    pub separators: Separators,
    /// Segments in order, starting with MSH.
    pub segments: Vec<Segment>,
}

impl Message {
    /// The first segment with the given id.
    pub fn segment(&self, id: &str) -> Option<&Segment> {
        self.segments.iter().find(|s| s.id == id)
    }

    /// All segments with the given id, in order (for repeated segments such as `OBX`).
    pub fn segments_named<'a>(&'a self, id: &'a str) -> impl Iterator<Item = &'a Segment> + 'a {
        self.segments.iter().filter(move |s| s.id == id)
    }

    /// MSH-9, the message type, e.g. `ADT^A01`.
    pub fn message_type(&self) -> Option<String> {
        self.segment("MSH").and_then(|m| m.field(9)).map(|s| s.to_string())
    }

    /// MSH-10, the message control id.
    pub fn control_id(&self) -> Option<String> {
        self.segment("MSH").and_then(|m| m.field(10)).map(|s| s.to_string())
    }
}

/// Parse an HL7 v2 message. Segments may be separated by carriage returns or line feeds.
pub fn parse(input: &[u8]) -> Result<Message, Hl7Error> {
    let text = core::str::from_utf8(input)
        .map_err(|e| tpt_io_core::ParseError::custom(e.valid_up_to(), "message is not valid UTF-8"))?;
    let text = text.trim_start_matches(['\r', '\n', ' ']);
    if !text.starts_with("MSH") {
        return Err(Hl7Error::Invalid("the message must start with MSH".to_string()));
    }
    let field_sep = text[3..].chars().next().ok_or_else(|| Hl7Error::Invalid("MSH has no separator".to_string()))?;
    let enc: Vec<char> = text[3 + field_sep.len_utf8()..].chars().take(4).collect();
    if enc.len() < 4 {
        return Err(Hl7Error::Invalid("MSH-2 must hold four encoding characters".to_string()));
    }
    let separators = Separators {
        field: field_sep,
        component: enc[0],
        repetition: enc[1],
        escape: enc[2],
        subcomponent: enc[3],
    };
    let mut segments = Vec::new();
    for line in text.split(['\r', '\n']) {
        if line.trim().is_empty() {
            continue;
        }
        let mut parts = line.split(field_sep);
        let id = parts.next().unwrap_or("").to_string();
        if id.len() != 3 || !id.bytes().all(|b| b.is_ascii_uppercase() || b.is_ascii_digit()) {
            return Err(Hl7Error::Invalid(alloc::format!("bad segment id '{id}'")));
        }
        let fields = parts.map(|p| p.to_string()).collect();
        segments.push(Segment { id, fields });
    }
    Ok(Message { separators, segments })
}

/// Serialise a message with carriage-return segment terminators.
pub fn write(m: &Message) -> Vec<u8> {
    let sep = m.separators.field;
    let mut out = String::new();
    for s in &m.segments {
        out.push_str(&s.id);
        for f in &s.fields {
            out.push(sep);
            out.push_str(f);
        }
        out.push('\r');
    }
    out.into_bytes()
}
