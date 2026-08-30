//! DICOM data model: tags, values, datasets, and files.

use alloc::string::String;
use alloc::vec::Vec;

use crate::tag::VRCode;

/// A DICOM tag: `(group, element)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Tag(pub u16, pub u16);

impl Tag {
    /// Construct a new tag from group and element numbers.
    pub const fn new(group: u16, element: u16) -> Self {
        Self(group, element)
    }

    /// The group number.
    pub const fn group(self) -> u16 {
        self.0
    }

    /// The element number.
    pub const fn element(self) -> u16 {
        self.1
    }

    /// Whether this is a private tag (odd group number).
    pub const fn is_private(self) -> bool {
        self.0 & 1 == 1
    }

    /// Whether this is a group-length tag (element == 0x0000).
    pub const fn is_group_length(self) -> bool {
        self.1 == 0x0000
    }
}

/// A DICOM value representation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VR {
    /// A known VR code.
    Known(VRCode),
    /// Unknown / private VR (stored as raw bytes).
    Unknown([u8; 2]),
}

impl VR {
    /// The VR code if known.
    pub const fn code(self) -> Option<VRCode> {
        match self {
            VR::Known(c) => Some(c),
            VR::Unknown(_) => None,
        }
    }

    /// Whether this VR is a sequence (SQ).
    pub const fn is_sequence(&self) -> bool {
        matches!(self, VR::Known(VRCode::SQ))
    }
}

impl core::fmt::Display for VR {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            VR::Known(c) => {
                let b = c.as_bytes();
                write!(f, "{}{}", b[0] as char, b[1] as char)
            }
            VR::Unknown(b) => write!(f, "{}{}", b[0] as char, b[1] as char),
        }
    }
}

/// A single DICOM element value.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    /// Raw bytes (for OB, OW, OD, OF, OL, OV, UN, etc.)
    Raw(Vec<u8>),
    /// UTF-8 / ASCII string.
    String(String),
    /// Signed 32-bit integer.
    I32(i32),
    /// Unsigned 32-bit integer.
    U32(u32),
    /// Signed 16-bit integer.
    I16(i16),
    /// Unsigned 16-bit integer.
    U16(u16),
    /// 32-bit float.
    F32(f32),
    /// 64-bit float.
    F64(f64),
    /// Sequence of items (nested datasets).
    Sequence(Vec<Dataset>),
}

/// A DICOM dataset: an ordered list of (tag, VR, value) elements.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Dataset {
    /// Ordered elements.
    pub elements: Vec<Element>,
}

/// A single DICOM element: tag + VR + value.
#[derive(Debug, Clone, PartialEq)]
pub struct Element {
    /// The DICOM tag.
    pub tag: Tag,
    /// The Value Representation.
    pub vr: VR,
    /// The value.
    pub value: Value,
}

impl Dataset {
    /// Create an empty dataset.
    pub fn new() -> Self {
        Self { elements: Vec::new() }
    }

    /// Get the first element matching the given tag.
    pub fn get(&self, tag: Tag) -> Option<&Element> {
        self.elements.iter().find(|e| e.tag == tag)
    }

    /// Get the value of the first element matching the given tag.
    pub fn value(&self, tag: Tag) -> Option<&Value> {
        self.get(tag).map(|e| &e.value)
    }

    /// Get the string value of the first element matching the given tag.
    pub fn string(&self, tag: Tag) -> Option<&str> {
        match self.value(tag)? {
            Value::String(s) => Some(s.as_str()),
            _ => None,
        }
    }

    /// Get the raw bytes value of the first element matching the given tag.
    pub fn raw(&self, tag: Tag) -> Option<&[u8]> {
        match self.value(tag)? {
            Value::Raw(b) => Some(b.as_slice()),
            _ => None,
        }
    }

    /// Add an element to the dataset.
    pub fn push(&mut self, element: Element) {
        self.elements.push(element);
    }

    /// Get the number of elements.
    pub fn len(&self) -> usize {
        self.elements.len()
    }

    /// Whether the dataset is empty.
    pub fn is_empty(&self) -> bool {
        self.elements.is_empty()
    }
}

/// A parsed DICOM file: file meta-information + data set.
#[derive(Debug, Clone, PartialEq)]
pub struct DicomFile {
    /// File meta-information group (0002,xxxx).
    pub meta: Dataset,
    /// Main data set.
    pub dataset: Dataset,
    /// The detected transfer syntax.
    pub transfer_syntax: crate::uid::TransferSyntax,
}

/// Build a DicomFile from its parts.
pub fn from_parts(
    meta: Dataset,
    dataset: Dataset,
    transfer_syntax: crate::uid::TransferSyntax,
) -> DicomFile {
    DicomFile { meta, dataset, transfer_syntax }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tag_is_private() {
        assert!(Tag::new(0x0009, 0x0010).is_private());
        assert!(!Tag::new(0x0008, 0x0016).is_private());
    }

    #[test]
    fn dataset_get_string() {
        let mut ds = Dataset::new();
        ds.push(Element {
            tag: Tag::new(0x0008, 0x0016),
            vr: VR::Known(VRCode::UI),
            value: Value::String("1.2.840.10008.5.1.4.1.1.2".into()),
        });
        assert_eq!(ds.string(Tag::new(0x0008, 0x0016)), Some("1.2.840.10008.5.1.4.1.1.2"));
        assert_eq!(ds.string(Tag::new(0x0008, 0x0018)), None);
    }

    #[test]
    fn vr_display() {
        let vr = VR::Known(VRCode::UI);
        assert_eq!(alloc::format!("{vr}"), "UI");
    }
}
