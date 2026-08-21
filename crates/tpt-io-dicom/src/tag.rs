//! Tag type — the `(group, element)` pair identifying every DICOM data
//! element (PS3.7 §6.4 / PS3.6).

use core::fmt;

/// A DICOM data element tag: 16-bit group and 16-bit element numbers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Tag {
    /// Group number (`gggg`).
    pub group: u16,
    /// Element number (`eeee`).
    pub element: u16,
}

impl Tag {
    /// Construct a tag from its components.
    pub const fn new(group: u16, element: u16) -> Self {
        Self { group, element }
    }

    /// File Meta Information group.
    pub const META_GROUP: u16 = 0x0002;
    /// Command set group (PS3.7).
    pub const COMMAND_GROUP: u16 = 0x0000;
    /// Item tag `(FFFE,E000)`.
    pub const ITEM: Tag = Tag::new(0xFFFE, 0xE000);
    /// Item delimiter `(FFFE,E00D)`.
    pub const ITEM_DELIMITATION: Tag = Tag::new(0xFFFE, 0xE00D);
    /// Sequence delimiter `(FFFE,E0DD)`.
    pub const SEQUENCE_DELIMITATION: Tag = Tag::new(0xFFFE, 0xE0DD);

    /// Whether this is a private-data-element tag (odd group, PS3.5 §7.1).
    pub const fn is_private(&self) -> bool {
        self.group & 1 == 1
    }

    /// Whether this is a delimiter tag in group `FFFE`.
    pub const fn is_delimiter(&self) -> bool {
        self.group == 0xFFFE
    }

    /// Whether this belongs to the File Meta Information group `0002`
    /// (always Explicit VR Little Endian, PS3.10 §7.1).
    pub const fn is_meta_group(&self) -> bool {
        self.group == Self::META_GROUP
    }

    /// Whether this belongs to the Command Set group `0000` (always Implicit
    /// VR Little Endian with always-even lengths, PS3.7 §6.3).
    pub const fn is_command_group(&self) -> bool {
        self.group == Self::COMMAND_GROUP
    }

    /// Parse from the canonical `"(gggg,eeee)"` string form.
    pub fn parse(s: &str) -> Option<Self> {
        let s = s.trim().trim_start_matches('(').trim_end_matches(')');
        let (g, e) = s.split_once(',')?;
        Some(Tag::new(
            u16::from_str_radix(g.trim(), 16).ok()?,
            u16::from_str_radix(e.trim(), 16).ok()?,
        ))
    }
}

impl fmt::Display for Tag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({:04x},{:04x})", self.group, self.element)
    }
}

impl From<(u16, u16)> for Tag {
    fn from((g, e): (u16, u16)) -> Self {
        Tag::new(g, e)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_and_parse_roundtrip() {
        for t in [
            Tag::new(0x0010, 0x0010),
            Tag::ITEM,
            Tag::new(0x7FE0, 0x0010),
            Tag::new(0x0002, 0x0010),
        ] {
            assert_eq!(Tag::parse(&t.to_string()), Some(t));
        }
    }

    #[test]
    fn private_and_delimiters() {
        assert!(Tag::new(0x0009, 0x0001).is_private());
        assert!(!Tag::new(0x0008, 0x0018).is_private());
        assert!(Tag::ITEM.is_delimiter());
        assert!(!Tag::new(0x7FE0, 0x0010).is_delimiter());
        assert!(Tag::new(0x0002, 0x0000).is_meta_group());
        assert!(Tag::new(0x0000, 0x0100).is_command_group());
    }

    #[test]
    fn parse_rejects_garbage() {
        assert_eq!(Tag::parse("not-a-tag"), None);
        assert_eq!(Tag::parse("(zzzz,eeee)"), None);
    }
}
