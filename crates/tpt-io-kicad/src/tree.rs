//! Generic S-expression tree for KiCad files.
//!
//! The tokens come from [`tpt_io_core::sexpr`]. This module nests them into
//! [`Node`]s and offers the lookups that the typed extraction in `model` needs.

use alloc::string::{String, ToString};
use alloc::vec::Vec;

use tpt_io_core::sexpr::{tokenize, Located, Token};
use tpt_io_core::ParseError;

use crate::error::KicadError;

/// A node of a KiCad S-expression.
#[derive(Debug, Clone, PartialEq)]
pub enum Node {
    /// A bare word such as `kicad_pcb` or `yes`.
    Symbol(String),
    /// A number.
    Number(f64),
    /// A quoted string.
    Str(String),
    /// A parenthesised list.
    List(Vec<Node>),
}

impl Node {
    /// The items of a list (empty for atoms).
    pub fn items(&self) -> &[Node] {
        match self {
            Node::List(items) => items,
            _ => &[],
        }
    }

    /// The head symbol of a list, e.g. `footprint` for `(footprint …)`.
    pub fn head(&self) -> Option<&str> {
        match self.items().first() {
            Some(Node::Symbol(s)) => Some(s),
            _ => None,
        }
    }

    /// Child lists whose head is `name`, in document order.
    pub fn children<'a>(&'a self, name: &'a str) -> impl Iterator<Item = &'a Node> + 'a {
        self.items().iter().skip(1).filter(move |n| n.head() == Some(name))
    }

    /// The first child list whose head is `name`.
    pub fn find<'a>(&'a self, name: &str) -> Option<&'a Node> {
        self.items().iter().skip(1).find(|n| n.head() == Some(name))
    }

    /// The `i`-th argument, i.e. the item after the head (`0` is the first argument).
    pub fn arg(&self, i: usize) -> Option<&Node> {
        self.items().get(i + 1)
    }

    /// The `i`-th argument as text (a string or a symbol).
    pub fn arg_str(&self, i: usize) -> Option<&str> {
        match self.arg(i)? {
            Node::Str(s) | Node::Symbol(s) => Some(s),
            _ => None,
        }
    }

    /// The `i`-th argument as a number.
    pub fn arg_f64(&self, i: usize) -> Option<f64> {
        match self.arg(i)? {
            Node::Number(n) => Some(*n),
            _ => None,
        }
    }
}

fn err(offset: usize, reason: impl ToString) -> KicadError {
    KicadError::Core(ParseError::custom(offset, reason))
}

/// Parse a KiCad file into its root node.
///
/// A KiCad file is one top-level list. Atoms outside a list, unbalanced
/// parentheses, and a second top-level form are all errors.
pub fn parse_tree(input: &[u8]) -> Result<Node, KicadError> {
    let tokens: Vec<Located> = tokenize(input)?;
    let mut stack: Vec<(Vec<Node>, usize)> = Vec::new();
    let mut root: Option<Node> = None;

    for Located { tok, offset } in tokens {
        match tok {
            Token::LParen => stack.push((Vec::new(), offset)),
            Token::RParen => {
                let (items, _) = stack.pop().ok_or_else(|| err(offset, "unbalanced ')'"))?;
                let node = Node::List(items);
                match stack.last_mut() {
                    Some((parent, _)) => parent.push(node),
                    None if root.is_none() => root = Some(node),
                    None => return Err(err(offset, "more than one top-level form")),
                }
            }
            atom => {
                let node = match atom {
                    Token::Symbol(s) => Node::Symbol(s),
                    Token::Number(n) => Node::Number(n),
                    Token::Str(s) => Node::Str(s),
                    _ => unreachable!("parens are handled above"),
                };
                match stack.last_mut() {
                    Some((items, _)) => items.push(node),
                    None => return Err(err(offset, "value outside a list")),
                }
            }
        }
    }

    if let Some((_, open)) = stack.last() {
        return Err(err(*open, "unclosed '('"));
    }
    root.ok_or_else(|| err(0, "empty document"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nests_lists_and_reads_args() {
        let root = parse_tree(b"(net 1 \"GND\" (extra yes))").unwrap();
        assert_eq!(root.head(), Some("net"));
        assert_eq!(root.arg_f64(0), Some(1.0));
        assert_eq!(root.arg_str(1), Some("GND"));
        assert_eq!(root.find("extra").and_then(|n| n.arg_str(0)), Some("yes"));
    }

    #[test]
    fn structure_errors_carry_offsets() {
        let open = parse_tree(b"(a (b c)").unwrap_err();
        assert!(matches!(open, KicadError::Core(e) if e.offset == 0));
        let close = parse_tree(b"(a))").unwrap_err();
        assert!(matches!(close, KicadError::Core(e) if e.offset == 3));
        assert!(parse_tree(b"x (a)").is_err());
        assert!(parse_tree(b"(a) (b)").is_err());
        assert!(parse_tree(b"").is_err());
    }
}
