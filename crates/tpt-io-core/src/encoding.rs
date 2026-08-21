//! Character-encoding detection from a raw byte slice (BOM + heuristic).
//!
//! Detection is `no_std`-friendly: it never allocates and works on an arbitrary
//! `&[u8]`. Only ASCII and UTF-8 inputs can be exposed as `&str` without
//! transcoding; UTF-16 inputs are reported by enum variant so callers can
//! decide whether to transcode.

/// A character encoding detected from a byte slice.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Encoding {
    /// Pure 7-bit ASCII (a subset of UTF-8).
    Ascii,
    /// UTF-8 (optionally with a UTF-8 BOM).
    Utf8,
    /// UTF-16 little-endian (optionally with a UTF-16LE BOM).
    Utf16Le,
    /// UTF-16 big-endian (optionally with a UTF-16BE BOM).
    Utf16Be,
}

impl Encoding {
    /// Detect the most likely encoding of `bytes`.
    ///
    /// Order of precedence: explicit byte-order mark, then a structural
    /// heuristic (pure-ASCII → UTF-8 validity → UTF-16 NUL-pattern → UTF-8
    /// fallback).
    pub fn detect(bytes: &[u8]) -> Encoding {
        // 1. Explicit BOMs win.
        if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
            return Encoding::Utf8;
        }
        if bytes.starts_with(&[0xFF, 0xFE]) {
            return Encoding::Utf16Le;
        }
        if bytes.starts_with(&[0xFE, 0xFF]) {
            return Encoding::Utf16Be;
        }

        // 2. Every byte < 0x80 ⇒ pure ASCII.
        if bytes.iter().all(|b| *b < 0x80) {
            return Encoding::Ascii;
        }

        // 3. Valid UTF-8 byte sequence ⇒ UTF-8.
        if core::str::from_utf8(bytes).is_ok() {
            return Encoding::Utf8;
        }

        // 4. Even-length data with a regular NUL high/low byte ⇒ UTF-16.
        if bytes.len() % 2 == 0 && bytes.len() >= 2 {
            let all_even_nul = bytes.iter().step_by(2).all(|b| *b == 0);
            let all_odd_nul = bytes.iter().skip(1).step_by(2).all(|b| *b == 0);
            if all_even_nul {
                return Encoding::Utf16Be; // high bytes zero ⇒ BE
            }
            if all_odd_nul {
                return Encoding::Utf16Le; // low bytes zero ⇒ LE
            }
        }

        // 5. Fallback: treat as UTF-8 (best-effort for mixed content).
        Encoding::Utf8
    }

    /// Whether this encoding can be interpreted as `&str` without transcoding.
    pub fn is_str_safe(self) -> bool {
        matches!(self, Encoding::Ascii | Encoding::Utf8)
    }

    /// Return the input as `&str` if the encoding is ASCII or UTF-8, stripping a
    /// leading UTF-8 BOM if present. Returns `None` for UTF-16 inputs.
    pub fn as_str(self, bytes: &[u8]) -> Option<&str> {
        if !self.is_str_safe() {
            return None;
        }
        let stripped = if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) { &bytes[3..] } else { bytes };
        core::str::from_utf8(stripped).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_ascii() {
        assert_eq!(Encoding::detect(b"hello"), Encoding::Ascii);
    }

    #[test]
    fn detects_utf8_bom() {
        assert_eq!(Encoding::detect(&[0xEF, 0xBB, 0xBF, b'h', b'i']), Encoding::Utf8);
    }

    #[test]
    fn detects_utf8_multibyte() {
        assert_eq!(Encoding::detect("héllo".as_bytes()), Encoding::Utf8);
    }

    #[test]
    fn detects_utf16le_bom() {
        assert_eq!(Encoding::detect(&[0xFF, 0xFE, 0x48, 0x00]), Encoding::Utf16Le);
    }

    #[test]
    fn detects_utf16be_bom() {
        assert_eq!(Encoding::detect(&[0xFE, 0xFF, 0x00, 0x48]), Encoding::Utf16Be);
    }

    #[test]
    fn strips_utf8_bom() {
        let bytes = [0xEF, 0xBB, 0xBF, b'h', b'i'];
        assert_eq!(Encoding::detect(&bytes).as_str(&bytes), Some("hi"));
    }

    #[test]
    fn utf16_not_str_safe() {
        let bytes = [0xFF, 0xFE, 0x48, 0x00];
        assert_eq!(Encoding::detect(&bytes).as_str(&bytes), None);
    }
}
