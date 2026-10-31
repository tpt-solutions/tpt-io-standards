//! G-code model, parser, writer and motion summary.

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::fmt::Write as _;

use tpt_io_core::ParseError;

use crate::error::GcodeError;

/// The comment style a block used.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Comment {
    /// `;` to the end of the line.
    Semicolon(String),
    /// `(…)` anywhere on the line.
    Paren(String),
}

/// One word: a letter and its number. `text` is the word as written (for example `X-1.50`).
#[derive(Debug, Clone, PartialEq)]
pub struct Word {
    /// The address letter, uppercase.
    pub letter: char,
    /// The numeric value.
    pub value: f64,
    /// The word as written in the source.
    pub text: String,
}

/// One line of a program.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Block {
    /// The line number word (`N10`), if present.
    pub line_number: Option<u32>,
    /// The words in order, excluding the line number.
    pub words: Vec<Word>,
    /// The comment on the line, if any.
    pub comment: Option<Comment>,
}

impl Block {
    /// The first word with the given letter.
    pub fn word(&self, letter: char) -> Option<&Word> {
        self.words.iter().find(|w| w.letter == letter)
    }
}

/// The controller family a program appears to target.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dialect {
    /// Semicolon comments and no program-number words: RepRap and similar firmware.
    Reprap,
    /// Parenthesised comments or `O` program numbers: LinuxCNC and Fanuc-style controllers.
    LinuxCnc,
    /// Neither style is evident.
    Generic,
}

/// A parsed program.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Program {
    /// Blocks in file order. Blank lines and `%` delimiters are not kept.
    pub blocks: Vec<Block>,
}

/// Results of walking a program's motion commands.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct MotionSummary {
    /// Number of `G0` rapid moves.
    pub rapid_moves: usize,
    /// Number of `G1` linear feed moves.
    pub linear_moves: usize,
    /// Number of `G2`/`G3` arc moves.
    pub arc_moves: usize,
    /// Path length of rapid and linear moves in the XYZ axes, in millimetres. Arcs are not
    /// measured.
    pub path_length_mm: f64,
    /// Minimum and maximum X and Y reached, in millimetres, if any move set them.
    pub xy_bounds: Option<[f64; 4]>,
}

impl Program {
    /// Detect the controller dialect from the comment styles and words present.
    pub fn dialect(&self) -> Dialect {
        let mut paren = false;
        let mut semi = false;
        let mut program_number = false;
        for b in &self.blocks {
            match &b.comment {
                Some(Comment::Paren(_)) => paren = true,
                Some(Comment::Semicolon(_)) => semi = true,
                None => {}
            }
            if b.words.iter().any(|w| w.letter == 'O') {
                program_number = true;
            }
        }
        if paren || program_number {
            Dialect::LinuxCnc
        } else if semi {
            Dialect::Reprap
        } else {
            Dialect::Generic
        }
    }

    /// Walk the program's modal state and summarise its motion.
    pub fn motion_summary(&self) -> MotionSummary {
        let mut s = MotionSummary::default();
        let mut pos = [0.0f64; 3];
        let mut absolute = true;
        let mut inches = false;
        let mut mode = 0u32;
        let mut bounds: Option<[f64; 4]> = None;
        for b in &self.blocks {
            for w in b.words.iter().filter(|w| w.letter == 'G') {
                match w.value as u32 {
                    0..=3 => mode = w.value as u32,
                    90 => absolute = true,
                    91 => absolute = false,
                    20 => inches = true,
                    21 => inches = false,
                    _ => {}
                }
            }
            let target_axes: Vec<(usize, f64)> = ['X', 'Y', 'Z']
                .iter()
                .enumerate()
                .filter_map(|(i, axis)| b.word(*axis).map(|w| (i, w.value)))
                .collect();
            if target_axes.is_empty() {
                continue;
            }
            match mode {
                0 => s.rapid_moves += 1,
                1 => s.linear_moves += 1,
                _ => s.arc_moves += 1,
            }
            // Arc endpoints still move the tool, so the position updates; only the length is skipped.
            let scale = if inches { 25.4 } else { 1.0 };
            let mut next = pos;
            for (i, v) in target_axes {
                let v = v * scale;
                next[i] = if absolute { v } else { pos[i] + v };
            }
            if mode <= 1 {
                let d = ((next[0] - pos[0]).powi(2) + (next[1] - pos[1]).powi(2) + (next[2] - pos[2]).powi(2)).sqrt();
                s.path_length_mm += d;
            }
            pos = next;
            let b_xy = bounds.get_or_insert([pos[0], pos[0], pos[1], pos[1]]);
            b_xy[0] = b_xy[0].min(pos[0]);
            b_xy[1] = b_xy[1].max(pos[0]);
            b_xy[2] = b_xy[2].min(pos[1]);
            b_xy[3] = b_xy[3].max(pos[1]);
        }
        s.xy_bounds = bounds.map(|b| [b[0], b[1], b[2], b[3]]);
        s
    }
}

/// Parse a G-code program.
pub fn parse(input: &[u8]) -> Result<Program, GcodeError> {
    let text = core::str::from_utf8(input)
        .map_err(|e| ParseError::custom(e.valid_up_to(), "program is not valid UTF-8"))?;
    let mut blocks = Vec::new();
    let mut offset = 0usize;
    for line in text.split('\n') {
        let line_offset = offset;
        offset += line.len() + 1;
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed == "%" {
            continue;
        }
        blocks.push(block(line, line_offset)?);
    }
    Ok(Program { blocks })
}

fn block(line: &str, offset: usize) -> Result<Block, GcodeError> {
    let mut b = Block::default();
    let mut rest = line;
    // A `;` comment runs to the end of the line.
    let (code, semi) = match rest.find(';') {
        Some(i) => (&rest[..i], Some(rest[i + 1..].trim().to_string())),
        None => (rest, None),
    };
    rest = code;
    // A parenthesised comment is removed from the code and kept on the block.
    let mut paren: Option<String> = None;
    let mut cleaned = String::new();
    let mut i = 0usize;
    while i < rest.len() {
        let c = rest[i..].chars().next().unwrap_or(' ');
        if c == '(' {
            let body_start = i + 1;
            let close = rest[body_start..]
                .find(')')
                .ok_or_else(|| ParseError::custom(offset + i, "unterminated '(' comment"))?;
            paren = Some(rest[body_start..body_start + close].trim().to_string());
            i = body_start + close + 1;
            cleaned.push(' ');
        } else {
            cleaned.push(c);
            i += c.len_utf8();
        }
    }
    let mut first = true;
    for tok in cleaned.split_whitespace() {
        let letter = tok.chars().next().unwrap_or(' ').to_ascii_uppercase();
        let number_text = &tok[tok.chars().next().map_or(0, |c| c.len_utf8())..];
        if first && letter == 'N' {
            if let Ok(n) = number_text.parse::<u32>() {
                b.line_number = Some(n);
                first = false;
                continue;
            }
        }
        first = false;
        let value = number_text.parse::<f64>().map_err(|_| GcodeError::BadNumber {
            offset,
            text: tok.to_string(),
        })?;
        b.words.push(Word { letter, value, text: tok.to_string() });
    }
    b.comment = match (semi, paren) {
        (Some(s), _) => Some(Comment::Semicolon(s)),
        (None, Some(p)) => Some(Comment::Paren(p)),
        (None, None) => None,
    };
    Ok(b)
}

/// Serialise a program. Each block becomes one line: line number, words as written, then
/// the comment.
pub fn write(p: &Program) -> Vec<u8> {
    let mut out = String::new();
    for b in &p.blocks {
        let mut parts: Vec<String> = Vec::new();
        if let Some(n) = b.line_number {
            parts.push(alloc::format!("N{n}"));
        }
        for w in &b.words {
            parts.push(w.text.clone());
        }
        let mut line = parts.join(" ");
        match &b.comment {
            Some(Comment::Semicolon(c)) => {
                if !line.is_empty() {
                    line.push(' ');
                }
                let _ = write!(line, "; {c}");
            }
            Some(Comment::Paren(c)) => {
                if !line.is_empty() {
                    line.push(' ');
                }
                let _ = write!(line, "({c})");
            }
            None => {}
        }
        out.push_str(&line);
        out.push('\n');
    }
    out.into_bytes()
}
