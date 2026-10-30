//! Structured parse errors with byte-offset tracking, expected-vs-actual token
//! reporting, specification references, and surrounding-byte context.
//!
//! `ParseError` is `#![no_std]` compatible: it only requires `alloc` (the
//! `context` and `kind` payloads own their data). Domain crates build their own
//! error enums "on this shape" — typically by embedding a `ParseError` for the
//! common failure modes while adding format-specific variants.

use alloc::borrow::Cow;
use alloc::boxed::Box;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::fmt;

/// A reference into the relevant section of a format specification, e.g.
/// `SpecReference::new("DICOM", "PS3.5 §6.2")`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SpecReference {
    /// Format family, e.g. `"DICOM"`, `"FHIR"`.
    pub format: &'static str,
    /// Specification section, e.g. `"PS3.5 §6.2"`.
    pub section: &'static str,
}

impl SpecReference {
    /// Construct a new specification reference.
    pub const fn new(format: &'static str, section: &'static str) -> Self {
        Self { format, section }
    }
}

impl fmt::Display for SpecReference {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.format, self.section)
    }
}

/// The token the parser expected versus what it actually found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TokenMismatch {
    /// What the parser expected (absent for "any token").
    pub expected: Option<Cow<'static, str>>,
    /// What was actually encountered at the error offset.
    pub actual: String,
}

impl TokenMismatch {
    /// Build a mismatch from `&str`/owned values.
    pub fn new<E, A>(expected: Option<E>, actual: A) -> Self
    where
        E: Into<Cow<'static, str>>,
        A: ToString,
    {
        Self { expected: expected.map(Into::into), actual: actual.to_string() }
    }
}

/// A bounded window of bytes surrounding the error offset, for debugging.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ErrorContext {
    /// Up to `window` bytes immediately before the error offset.
    pub preceding: Vec<u8>,
    /// Up to `window` bytes immediately after the error offset.
    pub following: Vec<u8>,
}

impl ErrorContext {
    /// Capture `window` bytes on each side of `offset` from `input`.
    pub fn capture_around(input: &[u8], offset: usize, window: usize) -> Self {
        let start = offset.saturating_sub(window);
        let end = core::cmp::min(offset.saturating_add(window), input.len());
        let pre_start = core::cmp::min(offset, input.len());
        Self {
            preceding: input[start..pre_start].to_vec(),
            following: input[pre_start..end].to_vec(),
        }
    }
}

/// The category of parse failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorKind {
    /// The input ended before the parser expected more data.
    UnexpectedEof,
    /// A single unexpected byte.
    InvalidByte {
        /// The offending byte value.
        byte: u8,
    },
    /// The parser expected one token but found another.
    TokenMismatch(TokenMismatch),
    /// The input is not valid in the assumed character encoding.
    InvalidEncoding,
    /// The format declares or uses a feature that is not (yet) supported.
    Unsupported(&'static str),
    /// Any other format-specific failure, with a human-readable reason.
    Custom(String),
}

/// The structured error returned by every `tpt-io` parser.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    /// Byte offset in the input stream where the error occurred.
    pub offset: usize,
    /// What went wrong.
    pub kind: ErrorKind,
    /// Optional link into the format specification.
    pub spec: Option<SpecReference>,
    /// Surrounding bytes for debugging.
    pub context: Box<ErrorContext>,
}

/// Convenience alias for results that fail with [`ParseError`].
pub type Result<T> = core::result::Result<T, ParseError>;

impl ParseError {
    /// Create an `UnexpectedEof` error at `offset`.
    pub fn eof(offset: usize) -> Self {
        Self {
            offset,
            kind: ErrorKind::UnexpectedEof,
            spec: None,
            context: Box::new(ErrorContext::default()),
        }
    }

    /// Create an `InvalidByte` error at `offset`.
    pub fn invalid_byte(offset: usize, byte: u8) -> Self {
        Self {
            offset,
            kind: ErrorKind::InvalidByte { byte },
            spec: None,
            context: Box::new(ErrorContext::default()),
        }
    }

    /// Create a `TokenMismatch` error at `offset`.
    pub fn token<M, A>(offset: usize, expected: Option<M>, actual: A) -> Self
    where
        M: Into<Cow<'static, str>>,
        A: ToString,
    {
        Self {
            offset,
            kind: ErrorKind::TokenMismatch(TokenMismatch::new(expected, actual)),
            spec: None,
            context: Box::new(ErrorContext::default()),
        }
    }

    /// Create an `Unsupported` error at `offset`.
    pub fn unsupported(offset: usize, feature: &'static str) -> Self {
        Self {
            offset,
            kind: ErrorKind::Unsupported(feature),
            spec: None,
            context: Box::new(ErrorContext::default()),
        }
    }

    /// Create a generic `Custom` error at `offset`.
    pub fn custom<A: ToString>(offset: usize, reason: A) -> Self {
        Self {
            offset,
            kind: ErrorKind::Custom(reason.to_string()),
            spec: None,
            context: Box::new(ErrorContext::default()),
        }
    }

    /// Attach a specification reference (builder-style).
    pub fn with_spec(mut self, spec: SpecReference) -> Self {
        self.spec = Some(spec);
        self
    }

    /// Capture surrounding bytes from `input` into the error context.
    pub fn with_context_from(mut self, input: &[u8], window: usize) -> Self {
        self.context = Box::new(ErrorContext::capture_around(input, self.offset, window));
        self
    }

    /// Map this error's offset (e.g. after seeking within a sub-buffer).
    pub fn at_offset(mut self, offset: usize) -> Self {
        self.offset = offset;
        self
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "parse error at byte {}: ", self.offset)?;
        match &self.kind {
            ErrorKind::UnexpectedEof => write!(f, "unexpected end of input")?,
            ErrorKind::InvalidByte { byte } => write!(f, "invalid byte 0x{byte:02x}")?,
            ErrorKind::TokenMismatch(tm) => match &tm.expected {
                Some(exp) => write!(f, "expected {exp}, found {}", tm.actual)?,
                None => write!(f, "unexpected token {}", tm.actual)?,
            },
            ErrorKind::InvalidEncoding => write!(f, "invalid character encoding")?,
            ErrorKind::Unsupported(feat) => write!(f, "unsupported feature: {feat}")?,
            ErrorKind::Custom(msg) => write!(f, "{msg}")?,
        }
        if let Some(spec) = &self.spec {
            write!(f, " [{spec}]")?;
        }
        Ok(())
    }
}

#[cfg(feature = "std")]
impl std::error::Error for ParseError {}
