//! Splitting an OpenDSS script into commands, and writing commands back.

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::fmt::Write as _;

use tpt_io_core::ParseError;

use crate::error::OpenDssError;

/// One logical command from the script.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Command {
    /// The leading word, e.g. `New`, `Edit`, `Set`, `Solve`, `Redirect`.
    pub verb: String,
    /// The element class for `New`/`Edit`/`More` commands, e.g. `Line`.
    pub object: Option<String>,
    /// The element name, e.g. `L1` in `New Line.L1`.
    pub name: Option<String>,
    /// Arguments that are not `key=value` pairs (for example a file name after `Redirect`).
    pub positional: Vec<String>,
    /// `key=value` properties in file order. Values are kept as written.
    pub props: Vec<(String, String)>,
    /// 1-based line number where the command starts.
    pub line: usize,
}

impl Command {
    /// The raw value of a property, matched case-insensitively.
    pub fn prop(&self, key: &str) -> Option<&str> {
        self.props
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(key))
            .map(|(_, v)| v.as_str())
    }
}

/// A parsed script: its commands in order.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Script {
    /// Commands in file order.
    pub commands: Vec<Command>,
}

impl Script {
    /// Commands that create or edit an element of the given class, e.g. `"Line"`.
    /// Matching is case-insensitive and includes `New` and `Edit` commands.
    pub fn elements<'a>(&'a self, class: &'a str) -> impl Iterator<Item = &'a Command> + 'a {
        self.commands.iter().filter(move |c| {
            c.object.as_deref().is_some_and(|o| o.eq_ignore_ascii_case(class))
        })
    }
}

/// Parse an OpenDSS script.
pub fn parse(input: &[u8]) -> Result<Script, OpenDssError> {
    let text = core::str::from_utf8(input)
        .map_err(|e| ParseError::custom(e.valid_up_to(), "script is not valid UTF-8"))?;
    let mut commands = Vec::new();
    let mut pending: Option<(usize, String)> = None;

    for (i, raw) in text.lines().enumerate() {
        let line_no = i + 1;
        let trimmed = raw.trim_start();
        if let Some(rest) = trimmed.strip_prefix('~') {
            // Continuation of the previous command.
            if let Some((_, buf)) = pending.as_mut() {
                buf.push(' ');
                buf.push_str(strip_comment(rest));
                continue;
            }
        }
        if let Some((start, buf)) = pending.take() {
            push_command(&mut commands, start, &buf)?;
        }
        pending = Some((line_no, strip_comment(raw).to_string()));
    }
    if let Some((start, buf)) = pending.take() {
        push_command(&mut commands, start, &buf)?;
    }
    Ok(Script { commands })
}

fn push_command(out: &mut Vec<Command>, line: usize, text: &str) -> Result<(), OpenDssError> {
    let tokens = tokenize(text, line)?;
    let mut iter = tokens.into_iter();
    let verb = match iter.next() {
        Some(v) => v,
        None => return Ok(()),
    };
    let lower = verb.to_ascii_lowercase();
    let takes_object = matches!(lower.as_str(), "new" | "edit" | "more");

    let mut object = None;
    let mut name = None;
    let mut positional = Vec::new();
    let mut props = Vec::new();

    let mut rest: Vec<String> = iter.collect();
    if takes_object && !rest.is_empty() && !is_property(&rest[0]) {
        let head = rest.remove(0);
        match head.split_once('.') {
            Some((class, n)) => {
                object = Some(class.to_string());
                name = Some(n.to_string());
            }
            None => object = Some(head),
        }
    }
    for tok in rest {
        match split_property(&tok) {
            Some((k, v)) => props.push((k.to_string(), v.to_string())),
            None => positional.push(tok),
        }
    }
    out.push(Command { verb, object, name, positional, props, line });
    Ok(())
}

/// Remove a `!` or `//` comment that starts outside quotes.
fn strip_comment(line: &str) -> &str {
    let mut in_quote = false;
    let bytes = line.as_bytes();
    for (i, c) in line.char_indices() {
        match c {
            '"' => in_quote = !in_quote,
            '!' if !in_quote => return &line[..i],
            '/' if !in_quote && bytes.get(i + 1) == Some(&b'/') => return &line[..i],
            _ => {}
        }
    }
    line
}

/// Split on whitespace and commas, except inside quotes, brackets or parentheses.
fn tokenize(text: &str, line: usize) -> Result<Vec<String>, OpenDssError> {
    let mut tokens = Vec::new();
    let mut cur = String::new();
    let mut depth = 0i32;
    let mut in_quote = false;
    for c in text.chars() {
        if in_quote {
            cur.push(c);
            if c == '"' {
                in_quote = false;
            }
            continue;
        }
        match c {
            '"' => {
                in_quote = true;
                cur.push(c);
            }
            '(' | '[' => {
                depth += 1;
                cur.push(c);
            }
            ')' | ']' => {
                depth -= 1;
                cur.push(c);
            }
            ' ' | '\t' | '\r' | ',' if depth == 0 => {
                if !cur.is_empty() {
                    tokens.push(core::mem::take(&mut cur));
                }
            }
            _ => cur.push(c),
        }
    }
    if in_quote || depth != 0 {
        return Err(ParseError::custom(line, "unterminated quote or bracket").into());
    }
    if !cur.is_empty() {
        tokens.push(cur);
    }
    Ok(tokens)
}

/// A `key=value` token has an `=` before any bracket or quote.
fn split_property(tok: &str) -> Option<(&str, &str)> {
    let eq = tok.find('=')?;
    if tok[..eq].contains(['(', '[', '"']) || eq == 0 {
        return None;
    }
    Some((&tok[..eq], &tok[eq + 1..]))
}

fn is_property(tok: &str) -> bool {
    split_property(tok).is_some()
}

/// Serialise a script. Each command is written on its own line with its raw property
/// values, so `parse(write(s))` gives back the same commands.
pub fn write(script: &Script) -> Vec<u8> {
    let mut out = String::new();
    for c in &script.commands {
        out.push_str(&c.verb);
        if let Some(obj) = &c.object {
            out.push(' ');
            out.push_str(obj);
            if let Some(n) = &c.name {
                out.push('.');
                out.push_str(n);
            }
        }
        for p in &c.positional {
            out.push(' ');
            out.push_str(p);
        }
        for (k, v) in &c.props {
            let _ = write!(out, " {k}={v}");
        }
        out.push('\n');
    }
    out.into_bytes()
}
