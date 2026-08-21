//! Core traits shared by every format parser in the workspace.
//!
//! These traits establish the uniform API described in `spec.txt §3` and
//! `§4`: `Reader`/`Writer` for (de)serialization, `Validator` for the
//! syntactic/semantic split, `Schema` for spec-linked structural checks, and
//! `FormatExtension` so downstream users can add custom tags without forking.
//!
//! `Reader`, `Validator`, `ValidationSettings`, and `Progress` are available in
//! a pure `no_std` (no `alloc`) build. The richer `Writer`/`Schema`/
//! `FormatExtension` traits need `alloc` because they own their output/errors.

/// Parse a complete input buffer into a strongly-typed value.
///
/// Implementors are typically zero- or low-allocation and `no_std`-compatible;
/// the associated [`Reader::Error`] lets each format choose its own rich error
/// type while still embedding [`crate::ParseError`] for the common cases.
pub trait Reader<T> {
    /// Error type returned on parse failure.
    type Error;

    /// Parse `input` into `T`.
    fn parse(&self, input: &[u8]) -> core::result::Result<T, Self::Error>;
}

/// Serialize a value back into bytes. Requires `alloc`/`std` (owned output).
#[cfg(any(feature = "alloc", feature = "std"))]
pub trait Writer<T> {
    /// Error type returned on serialization failure.
    type Error;

    /// Serialize `value` into an owned byte buffer.
    fn serialize(&self, value: &T) -> core::result::Result<alloc::vec::Vec<u8>, Self::Error>;
}

/// Two-phase validation: structural (syntactic) vs. meaningful (semantic).
///
/// Syntactic validation answers "is the file structurally valid?"; semantic
/// validation answers "does the content make sense?" (ranges, reference
/// resolution, …). Semantic checks are optional and can be skipped for trusted
/// inputs — see [`ValidationSettings`].
pub trait Validator<T> {
    /// Error type returned when validation fails.
    type Error;

    /// Check structural correctness.
    fn validate_syntactic(&self, value: &T) -> core::result::Result<(), Self::Error>;

    /// Check semantic correctness (may be skipped via [`ValidationSettings`]).
    fn validate_semantic(&self, value: &T) -> core::result::Result<(), Self::Error>;
}

/// A structural contract a parser can check against raw bytes or a typed value.
#[cfg(feature = "alloc")]
pub trait Schema {
    /// The specification section this schema is tied to.
    fn spec_reference(&self) -> crate::error::SpecReference;

    /// Validate raw `input` against this schema.
    fn validate_bytes(&self, input: &[u8]) -> crate::error::Result<()>;
}

/// Reusable validation configuration embedded by domain readers.
///
/// Mirrors the `with_syntactic_validation` / `with_semantic_validation`
/// builder style called for in `spec.txt §4`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ValidationSettings {
    syntactic: bool,
    semantic: bool,
}

impl ValidationSettings {
    /// Enable both syntactic and semantic validation.
    pub const fn new() -> Self {
        Self { syntactic: true, semantic: true }
    }

    /// Enable only syntactic validation (fast path for trusted input).
    pub const fn syntactic_only() -> Self {
        Self { syntactic: true, semantic: false }
    }

    /// Enable only semantic validation.
    pub const fn semantic_only() -> Self {
        Self { syntactic: false, semantic: true }
    }

    /// Builder: turn syntactic validation on/off.
    pub const fn with_syntactic_validation(mut self, enabled: bool) -> Self {
        self.syntactic = enabled;
        self
    }

    /// Builder: turn semantic validation on/off.
    pub const fn with_semantic_validation(mut self, enabled: bool) -> Self {
        self.semantic = enabled;
        self
    }

    /// Whether syntactic validation is enabled.
    pub const fn is_syntactic(&self) -> bool {
        self.syntactic
    }

    /// Whether semantic validation is enabled.
    pub const fn is_semantic(&self) -> bool {
        self.semantic
    }

    /// Run the appropriate checks for `value` through `validator`.
    pub fn run<V, T>(&self, validator: &V, value: &T) -> core::result::Result<(), V::Error>
    where
        V: Validator<T>,
    {
        if self.syntactic {
            validator.validate_syntactic(value)?;
        }
        if self.semantic {
            validator.validate_semantic(value)?;
        }
        Ok(())
    }
}

/// Progress-reporting hook for long-running / streaming parsers.
///
/// Implementors are notified periodically with the number of bytes consumed and
/// (optionally) the expected total, so callers can drive progress bars or
/// cancellation without the parser caring about the UI.
pub trait Progress {
    /// Report that `bytes_read` bytes have been consumed so far.
    fn on_progress(&self, bytes_read: u64, total: Option<u64>);
}

/// A no-op [`Progress`] implementation (the default for parsers).
#[derive(Debug, Clone, Copy, Default)]
pub struct NoProgress;

impl Progress for NoProgress {
    fn on_progress(&self, _bytes_read: u64, _total: Option<u64>) {}
}

/// Adapter turning a closure into a [`Progress`] implementation.
pub struct ProgressFn<F>(pub F)
where
    F: Fn(u64, Option<u64>);

impl<F> Progress for ProgressFn<F>
where
    F: Fn(u64, Option<u64>),
{
    fn on_progress(&self, bytes_read: u64, total: Option<u64>) {
        (self.0)(bytes_read, total);
    }
}

/// Extensibility hook allowing users to parse custom tags / dialects without
/// modifying the crate (see `spec.txt §4` Extensibility).
#[cfg(feature = "alloc")]
pub trait FormatExtension {
    /// The raw tag type the extension understands.
    type Tag;
    /// The value produced by the extension.
    type Value;

    /// Attempt to parse `tag` into a custom `Value`.
    fn parse_custom(&self, tag: &Self::Tag) -> crate::error::Result<Self::Value>;
}
