//! EDIF S-expression tokenizer.
//!
//! The lexer is shared with other S-expression formats and lives in
//! [`tpt_io_core::sexpr`]. This module re-exports it under the EDIF crate's paths.

pub use tpt_io_core::sexpr::{tokenize, Located, Token};
