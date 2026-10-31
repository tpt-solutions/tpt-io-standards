//! STAR tokenizer, data model, and writer for CIF and mmCIF.

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::fmt::Write as _;

use tpt_io_core::ParseError;

use crate::error::CifError;

/// A `_tag value` pair. `value` is the token as written (quotes kept) or the text-field body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Item {
    /// The tag, including its leading underscore.
    pub tag: String,
    /// The value. A text field is stored without its `;` delimiters and with a newline on each side.
    pub value: String,
    /// Whether the value was a text field (`;` … `;`).
    pub text_field: bool,
}

/// A `loop_` table: its tags and rows of values.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Loop {
    /// Column tags in order.
    pub tags: Vec<String>,
    /// Rows; each row has one value per tag.
    pub rows: Vec<Vec<String>>,
}

/// A `data_` block.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DataBlock {
    /// The block name after `data_`.
    pub name: String,
    /// `_tag value` items in file order.
    pub items: Vec<Item>,
    /// Loops in file order.
    pub loops: Vec<Loop>,
}

impl DataBlock {
    /// The value of an item, by tag (case-insensitive). A text field's value has no delimiters.
    pub fn item(&self, tag: &str) -> Option<&str> {
        self.items
            .iter()
            .find(|i| i.tag.eq_ignore_ascii_case(tag))
            .map(|i| i.value.as_str())
    }

    /// The loop that declares the given tag, if any.
    pub fn loop_with(&self, tag: &str) -> Option<&Loop> {
        self.loops.iter().find(|l| l.tags.iter().any(|t| t.eq_ignore_ascii_case(tag)))
    }
}

/// A CIF or mmCIF file: its data blocks.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CifFile {
    /// Blocks in file order.
    pub blocks: Vec<DataBlock>,
}

/// One token from the source.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Tok {
    Data(String),
    Text(String),
}

/// Parse a CIF or mmCIF file.
pub fn parse(input: &[u8]) -> Result<CifFile, CifError> {
    let src = core::str::from_utf8(input)
        .map_err(|e| ParseError::custom(e.valid_up_to(), "file is not valid UTF-8"))?;
    let tokens = tokenize(src)?;
    let mut file = CifFile::default();
    let mut i = 0usize;
    while i < tokens.len() {
        match &tokens[i] {
            Tok::Data(word) if is_data_header(word) => {
                file.blocks.push(DataBlock { name: word[5..].to_string(), ..DataBlock::default() });
                i += 1;
            }
            Tok::Data(word) if word.eq_ignore_ascii_case("loop_") => {
                let block = current(&mut file)?;
                i += 1;
                let mut lp = Loop::default();
                while let Some(Tok::Data(t)) = tokens.get(i) {
                    if !t.starts_with('_') {
                        break;
                    }
                    lp.tags.push(t.clone());
                    i += 1;
                }
                if lp.tags.is_empty() {
                    return Err(CifError::Invalid("loop_ with no tags".to_string()));
                }
                let width = lp.tags.len();
                let mut values = Vec::new();
                while let Some(tok) = tokens.get(i) {
                    let v = match tok {
                        Tok::Data(t) if t.starts_with('_') || is_data_header(t) || t.eq_ignore_ascii_case("loop_") => break,
                        Tok::Data(t) => t.clone(),
                        // A text cell is marked with one leading newline so writers can tell it apart.
                        Tok::Text(t) => alloc::format!("\n{t}"),
                    };
                    values.push(v);
                    i += 1;
                }
                if values.len() % width != 0 {
                    return Err(CifError::Invalid("loop values do not fill whole rows".to_string()));
                }
                lp.rows = values.chunks(width).map(|c| c.to_vec()).collect();
                block.loops.push(lp);
            }
            Tok::Data(word) if word.starts_with('_') => {
                let block = current(&mut file)?;
                let value = match tokens.get(i + 1) {
                    Some(Tok::Data(v)) if !v.starts_with('_') => {
                        i += 2;
                        (v.clone(), false)
                    }
                    Some(Tok::Text(t)) => {
                        i += 2;
                        (t.clone(), true)
                    }
                    _ => return Err(CifError::Invalid(alloc::format!("tag {word} has no value"))),
                };
                block.items.push(Item { tag: word.clone(), value: value.0, text_field: value.1 });
            }
            Tok::Data(word) => {
                return Err(ParseError::custom(0, alloc::format!("unexpected token {word}")).into());
            }
            Tok::Text(_) => {
                return Err(CifError::Invalid("text field without a tag".to_string()));
            }
        }
    }
    Ok(file)
}

/// A `data_` block header token.
fn is_data_header(word: &str) -> bool {
    word.get(..5).is_some_and(|p| p.eq_ignore_ascii_case("data_"))
}

fn current(file: &mut CifFile) -> Result<&mut DataBlock, CifError> {
    file.blocks
        .last_mut()
        .ok_or_else(|| CifError::Invalid("content before the first data_ block".to_string()))
}

/// Split the source into tokens: bare words, quoted strings (quotes kept), and `;` text fields.
fn tokenize(src: &str) -> Result<Vec<Tok>, CifError> {
    let mut out = Vec::new();
    let mut lines = src.split_inclusive('\n').peekable();
    let mut offset = 0usize;
    while let Some(line) = lines.next() {
        let line_offset = offset;
        offset += line.len();
        if line.starts_with(';') {
            // Text field: body runs until a line that starts with ';'.
            let mut body = String::new();
            // The newline right after the opening `;` is a delimiter, not part of the value.
            let first = line.strip_prefix(';').unwrap_or(line);
            if !first.trim_end_matches(['\r', '\n']).is_empty() {
                body.push_str(first);
            }
            let mut closed = false;
            for l in lines.by_ref() {
                offset += l.len();
                if l.starts_with(';') {
                    closed = true;
                    break;
                }
                body.push_str(l);
            }
            if !closed {
                return Err(ParseError::custom(line_offset, "unterminated text field").into());
            }
            out.push(Tok::Text(body.trim_end_matches(['\n', '\r']).to_string()));
            continue;
        }
        let mut rest = line.trim_end_matches(['\n', '\r']);
        loop {
            rest = rest.trim_start();
            if rest.is_empty() || rest.starts_with('#') {
                break;
            }
            let first = rest.chars().next().unwrap_or(' ');
            if first == '\'' || first == '"' {
                let close = rest[1..].find(first).ok_or_else(|| {
                    ParseError::custom(line_offset, "unterminated quoted value")
                })?;
                let end = close + 2;
                let after = rest[end..].chars().next();
                if after.is_some_and(|c| !c.is_whitespace()) {
                    // A quote that is not followed by a delimiter is part of a bare token.
                    let word_end = rest.find(char::is_whitespace).unwrap_or(rest.len());
                    out.push(Tok::Data(rest[..word_end].to_string()));
                    rest = &rest[word_end..];
                } else {
                    out.push(Tok::Data(rest[..end].to_string()));
                    rest = &rest[end..];
                }
            } else {
                let word_end = rest.find(char::is_whitespace).unwrap_or(rest.len());
                out.push(Tok::Data(rest[..word_end].to_string()));
                rest = &rest[word_end..];
            }
        }
    }
    Ok(out)
}

/// Serialise a file. Text fields are written with `;` delimiters on their own lines; other values are
/// written as they were read.
pub fn write(file: &CifFile) -> Vec<u8> {
    let mut out = String::new();
    for b in &file.blocks {
        let _ = writeln!(out, "data_{}", b.name);
        for it in &b.items {
            if it.text_field {
                let _ = writeln!(out, "{}", it.tag);
                write_text(&mut out, &it.value);
            } else {
                let _ = writeln!(out, "{} {}", it.tag, it.value);
            }
        }
        for lp in &b.loops {
            out.push_str("loop_
");
            for t in &lp.tags {
                let _ = writeln!(out, "{t}");
            }
            for row in &lp.rows {
                for cell in row {
                    match cell.strip_prefix('\n') {
                        Some(text) => write_text(&mut out, text),
                        None => {
                            out.push_str(cell);
                            out.push(' ');
                        }
                    }
                }
                out.push('\n');
            }
        }
    }
    out.into_bytes()
}

/// A text field: `;` opens it, the body follows, and a line holding `;` closes it.
fn write_text(out: &mut String, text: &str) {
    out.push_str(";\n");
    let _ = writeln!(out, "{text}");
    out.push_str(";\n");
}
