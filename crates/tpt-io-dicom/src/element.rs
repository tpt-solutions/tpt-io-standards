//! In-memory DICOM data model: [`DataElement`] and its [`Value`].

use alloc::string::{String, ToString};
use alloc::vec::Vec;

use crate::tag::Tag;
use crate::vr::Vr;

/// One item of a Sequence of Items (PS3.5 §7.5): a nested dataset.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Item {
    /// The elements contained in this item (its own dataset).
    pub elements: Vec<DataElement>,
}

impl Item {
    /// Get the first element with this exact tag.
    pub fn element(&self, tag: Tag) -> Option<&DataElement> {
        self.elements.iter().find(|e| e.tag == tag)
    }

    /// The element's value as trimmed text, if present.
    pub fn string(&self, tag: Tag) -> Option<String> {
        self.element(tag).and_then(DataElement::as_string)
    }

    /// The element's value as a `u16`, if present.
    pub fn u16_at(&self, tag: Tag) -> Option<u16> {
        self.element(tag).and_then(DataElement::as_u16)
    }

    /// The element's value as a `u32`, if present.
    pub fn u32_at(&self, tag: Tag) -> Option<u32> {
        self.element(tag).and_then(DataElement::as_u32)
    }
}

/// The value payload of a data element.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value {
    /// Raw encoded bytes (text, binary numbers, blobs, or — for
    /// encapsulated pixel data — the verbatim fragment item structure).
    Raw(Vec<u8>),
    /// A Sequence of Items (`VR = SQ`, PS3.5 §7.5).
    Sequence(Vec<Item>),
}

impl Value {
    /// The raw bytes of a [`Value::Raw`], if this is one.
    pub fn as_raw(&self) -> Option<&[u8]> {
        match self {
            Value::Raw(bytes) => Some(bytes),
            Value::Sequence(_) => None,
        }
    }

    /// Whether the value is empty (zero length).
    pub fn is_empty(&self) -> bool {
        match self {
            Value::Raw(b) => b.is_empty(),
            Value::Sequence(i) => i.is_empty(),
        }
    }
}

/// A single DICOM data element: a tag, its VR, and its value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataElement {
    /// The element tag `(gggg,eeee)`.
    pub tag: Tag,
    /// The Value Representation (known explicitly for Explicit VR syntaxes;
    /// inferred from the dictionary or set to [`Vr::Un`] for Implicit VR).
    pub vr: Vr,
    /// The value payload.
    pub value: Value,
}

impl DataElement {
    /// Build a raw-valued element.
    pub fn new(tag: Tag, vr: Vr, raw: Vec<u8>) -> Self {
        Self { tag, vr, value: Value::Raw(raw) }
    }

    /// Build a sequence-valued element.
    pub fn sequence(tag: Tag, items: Vec<Item>) -> Self {
        Self { tag, vr: Vr::Sq, value: Value::Sequence(items) }
    }

    /// Encode a `&str` value (appends nothing; caller handles padding rules).
    pub fn text(tag: Tag, vr: Vr, s: &str) -> Self {
        Self::new(tag, vr, s.as_bytes().to_vec())
    }

    /// Encode a single `u16` (US VR, little endian).
    pub fn us(tag: Tag, v: u16) -> Self {
        Self::new(tag, Vr::Us, v.to_le_bytes().to_vec())
    }

    /// Encode a single `u32` (UL VR, little endian).
    pub fn ul(tag: Tag, v: u32) -> Self {
        Self::new(tag, Vr::Ul, v.to_le_bytes().to_vec())
    }

    /// The value as trimmed UTF-8-ish text (trailing spaces/NULs stripped).
    ///
    /// DICOM character repertoires are a superset of ASCII; this returns the
    /// bytes decoded lossily so callers can treat most values as text.
    pub fn as_string(&self) -> Option<String> {
        let raw = self.value.as_raw()?;
        let trimmed: &[u8] =
            if raw.ends_with(b" ") || raw.ends_with(&[0]) { trim_end_ws(raw) } else { raw };
        // Lossy decode keeps invalid sequences inspectable instead of failing.
        Some(String::from_utf8_lossy(trimmed).to_string())
    }

    /// Interpret a binary US (unsigned short) value.
    pub fn as_u16(&self) -> Option<u16> {
        let raw = self.value.as_raw()?;
        if raw.len() < 2 {
            return None;
        }
        Some(u16::from_le_bytes([raw[0], raw[1]]))
    }

    /// Interpret a binary UL (unsigned long) value.
    pub fn as_u32(&self) -> Option<u32> {
        let raw = self.value.as_raw()?;
        if raw.len() < 4 {
            return None;
        }
        Some(u32::from_le_bytes([raw[0], raw[1], raw[2], raw[3]]))
    }

    /// Interpret an IS (Integer String) value.
    pub fn as_is(&self) -> Option<i64> {
        let s = self.as_string()?;
        s.trim().parse::<i64>().ok()
    }
}

fn trim_end_ws(mut b: &[u8]) -> &[u8] {
    while let Some((&last, rest)) = b.split_last() {
        if last == b' ' || last == 0 {
            b = rest;
        } else {
            break;
        }
    }
    b
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn us_ul_roundtrip() {
        let e = DataElement::us(Tag::new(0x0028, 0x0010), 512);
        assert_eq!(e.as_u16(), Some(512));
        let e = DataElement::ul(Tag::new(0x0002, 0x0000), 0xE000);
        assert_eq!(e.as_u32(), Some(0xE000));
        // Binary values are still lossily viewable as text.
        let t = DataElement::us(Tag::new(0x0028, 0x0010), 512);
        assert_eq!(t.as_u16(), Some(512));
        assert!(t.as_string().is_some());
    }

    #[test]
    fn text_trims_padding() {
        let e = DataElement::text(Tag::new(0x0010, 0x0020), Vr::Lo, "ID12345 ");
        assert_eq!(e.as_string().as_deref(), Some("ID12345"));
        let e = DataElement::new(Tag::new(0x0010, 0x0020), Vr::Lo, vec![b'A', 0, 0]);
        assert_eq!(e.as_string().as_deref(), Some("A"));
    }

    #[test]
    fn empty_values() {
        let e = DataElement::new(Tag::new(0x0008, 0x0050), Vr::Sh, Vec::new());
        assert!(e.value.is_empty());
        assert_eq!(e.as_string().as_deref(), Some(""));
    }

    #[test]
    fn is_integer_string() {
        let e = DataElement::text(Tag::new(0x0020, 0x0013), Vr::Is, "42");
        assert_eq!(e.as_is(), Some(42));
        let e = DataElement::text(Tag::new(0x0020, 0x0013), Vr::Is, " -7 ");
        assert_eq!(e.as_is(), Some(-7));
        let e = DataElement::text(Tag::new(0x0020, 0x0013), Vr::Is, "nope");
        assert_eq!(e.as_is(), None);
    }
}
