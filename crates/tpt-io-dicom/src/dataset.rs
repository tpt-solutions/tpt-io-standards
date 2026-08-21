//! Dataset parsing and serialization for the three native (uncompressed)
//! transfer syntaxes: Implicit VR LE, Explicit VR LE, and Explicit VR BE
//! (PS3.5 §7 and Annex A).
//!
//! Sequences of Items (PS3.5 §7.5) are parsed into a nested model; both
//! defined and undefined lengths are accepted on input and written as
//! defined lengths on output. Encapsulated (compressed) pixel data is kept
//! verbatim as raw bytes so it round-trips byte-for-byte and can be decoded
//! by the [`crate::pixel`] module.

use alloc::format;
use alloc::vec::Vec;

use crate::dict;
use crate::element::{DataElement, Item, Value};
use crate::error::{refs, DicomError, Result};
use crate::tag::Tag;
use crate::uid::TransferSyntax;
use crate::vr::Vr;

/// The `UNDEFINED_LENGTH` sentinel used in DICOM encodings (0xFFFFFFFF).
pub(crate) const UNDEFINED: u32 = 0xFFFF_FFFF;

/// Byte-order + VR-encoding parameters implied by a transfer syntax.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Encoding {
    /// Whether multi-byte values are big endian.
    pub big_endian: bool,
    /// Whether VRs are written explicitly after each tag.
    pub explicit_vr: bool,
}

impl Encoding {
    /// Encoding parameters for a transfer syntax. Returns [`None`] for the
    /// deflated variant (which needs a decompression stage first).
    pub fn for_transfer_syntax(ts: TransferSyntax) -> Option<Encoding> {
        Some(match ts {
            TransferSyntax::ImplicitVrLittleEndian => {
                Encoding { big_endian: false, explicit_vr: false }
            }
            TransferSyntax::ExplicitVrLittleEndian => {
                Encoding { big_endian: false, explicit_vr: true }
            }
            TransferSyntax::ExplicitVrBigEndian => Encoding { big_endian: true, explicit_vr: true },
            _ => return None,
        })
    }

    fn u16_at(&self, b: &[u8]) -> u16 {
        if self.big_endian {
            u16::from_be_bytes([b[0], b[1]])
        } else {
            u16::from_le_bytes([b[0], b[1]])
        }
    }

    fn u32_at(&self, b: &[u8]) -> u32 {
        if self.big_endian {
            u32::from_be_bytes([b[0], b[1], b[2], b[3]])
        } else {
            u32::from_le_bytes([b[0], b[1], b[2], b[3]])
        }
    }

    fn put_u16(&self, out: &mut Vec<u8>, v: u16) {
        let bytes = if self.big_endian { v.to_be_bytes() } else { v.to_le_bytes() };
        out.extend_from_slice(&bytes);
    }

    fn put_u32(&self, out: &mut Vec<u8>, v: u32) {
        let bytes = if self.big_endian { v.to_be_bytes() } else { v.to_le_bytes() };
        out.extend_from_slice(&bytes);
    }
}

// PART2

/// Parse a dataset (a flat list of elements) from `input[start..]`, stopping
/// at `end` (exclusive) or at a group-`FFFE` delimiter, whichever comes
/// first. Returns the elements and the offset just past the last one.
pub fn parse_dataset(
    input: &[u8],
    start: usize,
    end: usize,
    enc: Encoding,
) -> Result<(Vec<DataElement>, usize)> {
    let mut offset = start;
    let mut elements = Vec::new();
    while offset + 4 <= end {
        let group = enc.u16_at(&input[offset..]);
        let element = enc.u16_at(&input[offset + 2..]);
        if group == 0xFFFE {
            // Caller (sequence/item handling) owns delimiters; stop here.
            break;
        }
        let (elem, next) = parse_element(input, offset, end, enc, group, element)?;
        elements.push(elem);
        offset = next;
    }
    Ok((elements, offset))
}

/// Parse one element starting at `offset` with the tag already decoded.
///
/// Returns the element and the offset of the next element.
pub fn parse_element(
    input: &[u8],
    offset: usize,
    end: usize,
    enc: Encoding,
    group: u16,
    number: u16,
) -> Result<(DataElement, usize)> {
    let tag = Tag::new(group, number);
    let mut pos = offset + 4;

    // The File Meta group is *always* Explicit VR Little Endian regardless
    // of the transfer syntax (PS3.10 §7.1); Command sets are always Implicit.
    let elem_enc = if tag.is_meta_group() {
        Encoding { big_endian: false, explicit_vr: true }
    } else if tag.is_command_group() {
        Encoding { big_endian: false, explicit_vr: false }
    } else {
        enc
    };

    // Decode the VR (explicit) or infer it from the dictionary (implicit),
    // then locate the value bytes.
    let (vr, value_start, len) = if elem_enc.explicit_vr {
        let vr_bytes = slice(input, pos, 2)?;
        let parsed = Vr::from_bytes(vr_bytes).ok_or_else(|| {
            DicomError::Core(
                tpt_io_core::ParseError::token(
                    pos,
                    Some("valid VR code"),
                    alloc::format!("{:?}", vr_bytes),
                )
                .with_spec(refs::PS3_5)
                .with_context_from(input, 16),
            )
        })?;
        pos += 2;
        if parsed.uses_extended_length() {
            pos += 2; // two reserved bytes
            let len = elem_enc.u32_at(slice(input, pos, 4)?);
            pos += 4;
            (parsed, pos, len)
        } else {
            let len = elem_enc.u16_at(slice(input, pos, 2)?);
            pos += 2;
            (parsed, pos, u32::from(len))
        }
    } else {
        // Implicit VR: length is always u32; VR inferred from the dictionary.
        let len = elem_enc.u32_at(slice(input, pos, 4)?);
        pos += 4;
        let inferred = dict::vr_of(tag).unwrap_or(Vr::Un);
        (inferred, pos, len)
    };

    build_element(input, value_start, len, end, elem_enc, tag, vr)
}

/// Build the element value: sequences (SQ, or undefined length) are parsed
/// recursively; everything else is captured as raw bytes.
#[allow(clippy::too_many_arguments)]
fn build_element(
    input: &[u8],
    value_start: usize,
    len: u32,
    end: usize,
    enc: Encoding,
    tag: Tag,
    vr: Vr,
) -> Result<(DataElement, usize)> {
    if len == UNDEFINED {
        if vr == Vr::Sq {
            // Undefined-length sequence terminated by (FFFE,E0DD).
            let (items, next) = parse_sequence_undefined(input, value_start, enc)?;
            return Ok((DataElement { tag, vr: Vr::Sq, value: Value::Sequence(items) }, next));
        }
        // Undefined-length non-SQ (e.g. encapsulated Pixel Data): capture the
        // verbatim fragment structure so it round-trips and can be decoded.
        let stop =
            find_delimiter(input, value_start, Tag::SEQUENCE_DELIMITATION).ok_or_else(|| {
                DicomError::Core(tpt_io_core::ParseError::eof(input.len()).with_spec(refs::PS3_5))
            })?;
        let raw_end = stop + 8; // include the 8-byte sequence delimitation item
        bounds_check(input, raw_end)?;
        return Ok((
            DataElement { tag, vr, value: Value::Raw(input[value_start..stop].to_vec()) },
            raw_end,
        ));
    }
    let value_end = value_start + len as usize;
    bounds_check(input, value_end)?;
    if value_end > end {
        return Err(DicomError::Core(
            tpt_io_core::ParseError::custom(value_end, "element value exceeds enclosing length")
                .with_spec(refs::PS3_5),
        ));
    }
    if vr == Vr::Sq {
        // Defined-length sequence: parse items until the byte limit.
        let mut items = Vec::new();
        let mut pos = value_start;
        while pos + 8 <= value_end {
            let (item, next) = parse_item(input, pos, value_end, enc)?;
            items.push(item);
            pos = next;
        }
        if pos != value_end {
            return Err(DicomError::Core(
                tpt_io_core::ParseError::custom(pos, "misaligned defined-length sequence")
                    .with_spec(refs::PS3_5),
            ));
        }
        Ok((DataElement { tag, vr: Vr::Sq, value: Value::Sequence(items) }, value_end))
    } else {
        let mut raw = input[value_start..value_end].to_vec();
        normalize_padding(vr, &mut raw);
        Ok((DataElement::new(tag, vr, raw), value_end))
    }
}

/// Strip DICOM-defined trailing padding from text-valued elements so the
/// in-memory model is stable across parse → serialize → parse cycles.
/// Binary and blob VRs keep their exact bytes.
fn normalize_padding(vr: Vr, raw: &mut Vec<u8>) {
    if vr.binary_width().is_some()
        || matches!(vr, Vr::Ob | Vr::Od | Vr::Of | Vr::Ov | Vr::Ow | Vr::Un)
    {
        return;
    }
    while matches!(raw.last(), Some(b' ') | Some(0)) {
        raw.pop();
    }
}

/// Parse one Item `(FFFE,E000)` — either delimited by an explicit length or
/// by `(FFFE,E00D)` — into its nested elements.
pub fn parse_item(input: &[u8], offset: usize, end: usize, enc: Encoding) -> Result<(Item, usize)> {
    let group = enc.u16_at(slice(input, offset, 2)?);
    let number = enc.u16_at(slice(input, offset + 2, 2)?);
    if !(group == 0xFFFE && number == 0xE000) {
        return Err(DicomError::Core(
            tpt_io_core::ParseError::token(
                offset,
                Some("Item tag (fffe,e000)"),
                format!("({group:04x},{number:04x})"),
            )
            .with_spec(refs::PS3_5),
        ));
    }
    let len = enc.u32_at(slice(input, offset + 4, 4)?);
    let body_start = offset + 8;
    if len == UNDEFINED {
        let stop = find_delimiter(input, body_start, Tag::ITEM_DELIMITATION).ok_or_else(|| {
            DicomError::Core(tpt_io_core::ParseError::eof(input.len()).with_spec(refs::PS3_5))
        })?;
        let (elements, _) = parse_dataset(input, body_start, stop, enc)?;
        Ok((Item { elements }, stop + 8))
    } else {
        let body_end = body_start + len as usize;
        bounds_check(input, body_end)?;
        if body_end > end {
            return Err(DicomError::Core(
                tpt_io_core::ParseError::custom(body_end, "item length exceeds enclosing bound")
                    .with_spec(refs::PS3_5),
            ));
        }
        let (elements, consumed) = parse_dataset(input, body_start, body_end, enc)?;
        if consumed != body_end {
            return Err(DicomError::Core(
                tpt_io_core::ParseError::custom(consumed, "trailing bytes inside item")
                    .with_spec(refs::PS3_5),
            ));
        }
        Ok((Item { elements }, body_end))
    }
}

/// Parse an undefined-length sequence: items until `(FFFE,E0DD)`.
fn parse_sequence_undefined(
    input: &[u8],
    start: usize,
    enc: Encoding,
) -> Result<(Vec<Item>, usize)> {
    let mut items = Vec::new();
    let mut pos = start;
    loop {
        if pos + 8 > input.len() {
            return Err(DicomError::Core(tpt_io_core::ParseError::eof(pos).with_spec(refs::PS3_5)));
        }
        let group = enc.u16_at(&input[pos..]);
        let number = enc.u16_at(&input[pos + 2..]);
        if group == 0xFFFE && number == 0xE0DD {
            return Ok((items, pos + 8));
        }
        let (item, next) = parse_item(input, pos, input.len(), enc)?;
        items.push(item);
        pos = next;
    }
}

/// Serialize a list of elements in the given encoding. Sequences are always
/// written with defined lengths; undefined-length raw values (encapsulated
/// pixel data) are written back verbatim with their delimiters.
pub fn serialize_dataset(elements: &[DataElement], enc: Encoding, out: &mut Vec<u8>) -> Result<()> {
    for elem in elements {
        serialize_element(elem, enc, out)?;
    }
    Ok(())
}

/// Effective encoding for an element, honoring the always-explicit meta
/// group and always-implicit command group overrides.
fn effective_enc(elem: &DataElement, enc: Encoding) -> Encoding {
    if elem.tag.is_meta_group() {
        Encoding { big_endian: false, explicit_vr: true }
    } else if elem.tag.is_command_group() {
        Encoding { big_endian: false, explicit_vr: false }
    } else {
        enc
    }
}

/// Serialized length of one element's value, including even-length padding.
fn serialized_value_len(elem: &DataElement) -> usize {
    match &elem.value {
        Value::Raw(bytes) => {
            // Odd values are padded to even length on write (PS3.5 §7.1).
            // Encapsulated pixel data blobs are always even already.
            bytes.len() + usize::from(bytes.len() % 2 == 1)
        }
        Value::Sequence(items) => items
            .iter()
            .map(|item| 8 + item.elements.iter().map(serialized_value_len).sum::<usize>())
            .sum(),
    }
}

/// Full encoded length of one element under `enc` (header + value).
fn element_len(elem: &DataElement, enc: Encoding) -> usize {
    let e = effective_enc(elem, enc);
    let header = if e.explicit_vr && elem.vr.uses_extended_length() { 12 } else { 8 };
    header + serialized_value_len(elem)
}

fn serialize_element(elem: &DataElement, enc: Encoding, out: &mut Vec<u8>) -> Result<()> {
    let elem_enc = effective_enc(elem, enc);
    elem_enc.put_u16(out, elem.tag.group);
    elem_enc.put_u16(out, elem.tag.element);
    let vr = elem.vr;
    match &elem.value {
        Value::Raw(bytes) => {
            // Even-length padding (PS3.5 §7.1): the pad byte counts in the
            // declared length; UI pads with NUL, all other VRs with space.
            let padded_len = bytes.len() + usize::from(bytes.len() % 2 == 1);
            if elem_enc.explicit_vr {
                out.extend_from_slice(vr.as_bytes());
                if vr.uses_extended_length() {
                    out.extend_from_slice(&[0, 0]);
                    elem_enc.put_u32(out, padded_len as u32);
                } else {
                    if padded_len > u16::MAX as usize {
                        return Err(DicomError::invalid_value(
                            out.len(),
                            format!("value of {0} too long for short-form VR {vr}", elem.tag),
                        ));
                    }
                    elem_enc.put_u16(out, padded_len as u16);
                }
            } else {
                elem_enc.put_u32(out, padded_len as u32);
            }
            out.extend_from_slice(bytes);
            // Even-length padding (PS3.5 §7.1): UI pads with NUL, others space.
            if bytes.len() % 2 == 1 {
                out.push(if vr == Vr::Ui { 0 } else { b' ' });
            }
        }
        Value::Sequence(items) => {
            if elem_enc.explicit_vr {
                out.extend_from_slice(vr.as_bytes());
                out.extend_from_slice(&[0, 0]);
            }
            let body_len: usize = items
                .iter()
                .map(|i| 8 + i.elements.iter().map(|e| element_len(e, enc)).sum::<usize>())
                .sum();
            elem_enc.put_u32(out, body_len as u32);
            for item in items {
                serialize_item(item, elem_enc, out)?;
            }
        }
    }
    Ok(())
}

fn serialize_item(item: &Item, enc: Encoding, out: &mut Vec<u8>) -> Result<()> {
    enc.put_u16(out, 0xFFFE);
    enc.put_u16(out, 0xE000);
    let body_len: usize = item.elements.iter().map(|e| element_len(e, enc)).sum();
    enc.put_u32(out, body_len as u32);
    for e in &item.elements {
        serialize_element(e, enc, out)?;
    }
    Ok(())
}

/// Find the offset of an 8-byte delimiter item `(FFFE,xxxx)` in `input`
/// starting from `from`. Delimiters are always little endian per PS3.5 §7.5.
fn find_delimiter(input: &[u8], from: usize, tag: Tag) -> Option<usize> {
    let mut pos = from;
    while pos + 8 <= input.len() {
        if input[pos] == 0xFE
            && input[pos + 1] == 0xFF
            && input[pos + 2..pos + 4] == tag.element.to_le_bytes()
        {
            return Some(pos);
        }
        pos += 1;
    }
    None
}

fn slice(input: &[u8], offset: usize, len: usize) -> Result<&[u8]> {
    bounds_check(input, offset + len)?;
    Ok(&input[offset..offset + len])
}

fn bounds_check(input: &[u8], end: usize) -> Result<()> {
    if end > input.len() {
        return Err(DicomError::Core(tpt_io_core::ParseError::eof(end).with_spec(refs::PS3_5)));
    }
    Ok(())
}

// PART5

/// A parsed DICOM dataset (a flat, ordered list of elements) plus the
/// transfer syntax it was decoded from.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Dataset {
    /// Elements in tag order as they appeared.
    pub elements: Vec<DataElement>,
    /// The encoding the dataset was read with / should be written with.
    pub encoding: Encoding,
}

impl Dataset {
    /// Get the first element with this exact tag.
    pub fn element(&self, tag: Tag) -> Option<&DataElement> {
        self.elements.iter().find(|e| e.tag == tag)
    }

    /// The element's value as trimmed text, if present.
    pub fn string(&self, tag: Tag) -> Option<alloc::string::String> {
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

    /// The element's value as an IS integer, if present.
    pub fn is_at(&self, tag: Tag) -> Option<i64> {
        self.element(tag).and_then(DataElement::as_is)
    }

    /// Insert or replace an element preserving ascending tag order
    /// (the order required by PS3.5 §7.2).
    pub fn upsert(&mut self, elem: DataElement) {
        match self.elements.iter().position(|e| e.tag == elem.tag) {
            Some(i) => self.elements[i] = elem,
            None => {
                let pos = self.elements.partition_point(|e| e.tag < elem.tag);
                self.elements.insert(pos, elem);
            }
        }
    }

    /// Remove an element by tag, returning whether it was present.
    pub fn remove(&mut self, tag: Tag) -> bool {
        let before = self.elements.len();
        self.elements.retain(|e| e.tag != tag);
        self.elements.len() != before
    }
}

/// Parse a dataset in one of the native transfer syntaxes.
pub fn parse(input: &[u8], ts: TransferSyntax) -> Result<Dataset> {
    let enc = Encoding::for_transfer_syntax(ts)
        .ok_or_else(|| DicomError::UnsupportedTransferSyntax(ts.uid().into()))?;
    let (elements, _) = parse_dataset(input, 0, input.len(), enc)?;
    Ok(Dataset { elements, encoding: enc })
}

/// Serialize a dataset back into bytes using its recorded encoding.
pub fn serialize(dataset: &Dataset) -> Result<Vec<u8>> {
    let mut out = Vec::new();
    serialize_dataset(&dataset.elements, dataset.encoding, &mut out)?;
    Ok(out)
}

// PART6

#[cfg(test)]
mod tests {
    use super::*;

    use crate::element::Item;
    use crate::tag::Tag;
    use crate::vr::Vr;

    const T_PN: Tag = Tag::new(0x0010, 0x0010);
    const T_ID: Tag = Tag::new(0x0010, 0x0020);
    const T_ROWS: Tag = Tag::new(0x0028, 0x0010);

    fn sample_dataset() -> Dataset {
        let mut ds = Dataset {
            elements: Vec::new(),
            encoding: Encoding { big_endian: false, explicit_vr: true },
        };
        ds.upsert(DataElement::text(T_PN, Vr::Pn, "Doe^John"));
        ds.upsert(DataElement::text(T_ID, Vr::Lo, "PID-007")); // odd length → padding
        ds.upsert(DataElement::us(T_ROWS, 512));
        ds
    }

    fn ts_of(enc: Encoding) -> TransferSyntax {
        match (enc.big_endian, enc.explicit_vr) {
            (false, false) => TransferSyntax::ImplicitVrLittleEndian,
            (false, true) => TransferSyntax::ExplicitVrLittleEndian,
            (true, true) => TransferSyntax::ExplicitVrBigEndian,
            _ => unreachable!(),
        }
    }

    fn roundtrip(ds: &Dataset) -> Dataset {
        let bytes = serialize(ds).expect("serialize");
        parse(&bytes, ts_of(ds.encoding)).expect("parse")
    }

    // TESTS2

    #[test]
    fn explicit_le_roundtrip() {
        let ds = sample_dataset();
        let back = roundtrip(&ds);
        assert_eq!(back, ds);
        assert_eq!(back.string(T_PN).as_deref(), Some("Doe^John"));
        assert_eq!(back.u16_at(T_ROWS), Some(512));
    }

    #[test]
    fn implicit_le_roundtrip() {
        let mut ds = sample_dataset();
        ds.encoding = Encoding { big_endian: false, explicit_vr: false };
        let bytes = serialize(&ds).unwrap();
        let back = parse(&bytes, TransferSyntax::ImplicitVrLittleEndian).unwrap();
        assert_eq!(back.string(T_PN).as_deref(), Some("Doe^John"));
        assert_eq!(back.element(T_PN).unwrap().vr, Vr::Pn); // inferred from dictionary
        assert_eq!(back.u16_at(T_ROWS), Some(512));
    }

    #[test]
    fn explicit_be_roundtrip() {
        let mut ds = sample_dataset();
        ds.encoding = Encoding { big_endian: true, explicit_vr: true };
        let bytes = serialize(&ds).unwrap();
        let back = parse(&bytes, TransferSyntax::ExplicitVrBigEndian).unwrap();
        assert_eq!(back, ds);
    }

    // TESTS3

    #[test]
    fn sequence_defined_length_roundtrip() {
        let mut inner = Dataset {
            elements: Vec::new(),
            encoding: Encoding { big_endian: false, explicit_vr: true },
        };
        inner.upsert(DataElement::text(Tag::new(0x0008, 0x0060), Vr::Cs, "CT"));
        let seq_elem = DataElement::sequence(
            Tag::new(0x0008, 0x1140),
            vec![Item { elements: inner.elements }],
        );
        let mut ds = sample_dataset();
        ds.upsert(seq_elem);
        assert_eq!(roundtrip(&ds), ds);
    }

    #[test]
    fn undefined_length_sequence_roundtrip_model() {
        // Hand-encode an undefined-length SQ with two items (explicit LE).
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&0x0008u16.to_le_bytes());
        bytes.extend_from_slice(&0x1140u16.to_le_bytes());
        bytes.extend_from_slice(b"SQ");
        bytes.extend_from_slice(&[0, 0]);
        bytes.extend_from_slice(&UNDEFINED.to_le_bytes());
        for n in 0..2u16 {
            bytes.extend_from_slice(&0xFFFEu16.to_le_bytes());
            bytes.extend_from_slice(&0xE000u16.to_le_bytes());
            // item body: one US element (tag + VR + len=2 + u16 value)
            bytes.extend_from_slice(&10u32.to_le_bytes());
            bytes.extend_from_slice(&0x0028u16.to_le_bytes());
            bytes.extend_from_slice(&0x0011u16.to_le_bytes());
            bytes.extend_from_slice(b"US");
            bytes.extend_from_slice(&2u16.to_le_bytes());
            bytes.extend_from_slice(&n.to_le_bytes());
        }
        bytes.extend_from_slice(&0xFFFEu16.to_le_bytes());
        bytes.extend_from_slice(&0xE0DDu16.to_le_bytes());
        bytes.extend_from_slice(&0u32.to_le_bytes());

        let ds = parse(&bytes, TransferSyntax::ExplicitVrLittleEndian).unwrap();
        assert_eq!(ds.elements.len(), 1);
        match &ds.elements[0].value {
            Value::Sequence(items) => {
                assert_eq!(items.len(), 2);
                assert_eq!(items[1].u16_at(Tag::new(0x0028, 0x0011)), Some(1));
            }
            other => panic!("expected sequence, got {other:?}"),
        }
        // Re-serialization writes defined lengths; the model must survive.
        let out = serialize(&ds).unwrap();
        let back = parse(&out, TransferSyntax::ExplicitVrLittleEndian).unwrap();
        assert_eq!(back, ds);
    }

    // TESTS4

    #[test]
    fn encapsulated_pixel_data_roundtrip() {
        // Undefined-length OB value holding two fragment items + delimiter.
        let mut value = Vec::new();
        for frag in [&b"\x00\x01"[..], b"\x02\x03"] {
            value.extend_from_slice(&0xFFFEu16.to_le_bytes());
            value.extend_from_slice(&0xE000u16.to_le_bytes());
            value.extend_from_slice(&(frag.len() as u32).to_le_bytes());
            value.extend_from_slice(frag);
        }
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&0x7FE0u16.to_le_bytes());
        bytes.extend_from_slice(&0x0010u16.to_le_bytes());
        bytes.extend_from_slice(b"OB");
        bytes.extend_from_slice(&[0, 0]);
        bytes.extend_from_slice(&UNDEFINED.to_le_bytes());
        bytes.extend_from_slice(&value);
        bytes.extend_from_slice(&0xFFFEu16.to_le_bytes());
        bytes.extend_from_slice(&0xE0DDu16.to_le_bytes());
        bytes.extend_from_slice(&0u32.to_le_bytes());

        let ds = parse(&bytes, TransferSyntax::ExplicitVrLittleEndian).unwrap();
        let raw = ds.elements[0].value.as_raw().unwrap();
        assert_eq!(raw.len(), value.len()); // verbatim capture

        let out = serialize(&ds).unwrap();
        let back = parse(&out, TransferSyntax::ExplicitVrLittleEndian).unwrap();
        assert_eq!(back.elements[0].value.as_raw().unwrap(), raw);
    }

    #[test]
    fn truncated_input_errors_without_panic() {
        let ds = sample_dataset();
        let bytes = serialize(&ds).unwrap();
        for cut in [0usize, 2, 5, 7, 11] {
            let result = parse(&bytes[..cut], TransferSyntax::ExplicitVrLittleEndian);
            if cut < 4 {
                // Shorter than one tag header: an empty dataset, no error.
                assert_eq!(result.unwrap().elements.len(), 0);
            } else {
                assert!(result.is_err(), "expected error for truncation at {cut}");
            }
        }
    }

    // TESTS5

    #[test]
    fn invalid_vr_code_rejected() {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&0x0010u16.to_le_bytes());
        bytes.extend_from_slice(&0x0010u16.to_le_bytes());
        bytes.extend_from_slice(b"ZZ"); // invalid VR
        bytes.extend_from_slice(&4u16.to_le_bytes());
        bytes.extend_from_slice(b"ABCD");
        let err = parse(&bytes, TransferSyntax::ExplicitVrLittleEndian).unwrap_err();
        assert!(err.offset().is_some());
    }

    #[test]
    fn upsert_keeps_tag_order() {
        let mut ds = Dataset::default();
        ds.upsert(DataElement::us(T_ROWS, 1));
        ds.upsert(DataElement::text(T_PN, Vr::Pn, "A"));
        ds.upsert(DataElement::text(T_ID, Vr::Lo, "B"));
        let tags: Vec<Tag> = ds.elements.iter().map(|e| e.tag).collect();
        assert!(tags.windows(2).all(|w| w[0] < w[1]));
        ds.upsert(DataElement::us(T_ROWS, 99)); // replace in place
        assert_eq!(ds.elements.len(), 3);
        assert_eq!(ds.u16_at(T_ROWS), Some(99));
        assert!(ds.remove(T_ROWS));
        assert_eq!(ds.elements.len(), 2);
    }

    /// Deterministic xorshift64* PRNG driving property-style tests without
    /// an external dependency.
    pub(crate) struct Rng(u64);

    impl Rng {
        pub(crate) fn new(seed: u64) -> Self {
            Rng(seed | 1)
        }

        pub(crate) fn next_u32(&mut self) -> u32 {
            let mut x = self.0;
            x ^= x >> 12;
            x ^= x << 25;
            x ^= x >> 27;
            self.0 = x;
            (x.wrapping_mul(0x2545_F491_4F6C_DD1D) >> 32) as u32
        }

        pub(crate) fn below(&mut self, n: usize) -> usize {
            (self.next_u32() as usize) % n.max(1)
        }

        pub(crate) fn fill(&mut self, buf: &mut [u8]) {
            for chunk in buf.chunks_mut(4) {
                let v = self.next_u32().to_le_bytes();
                chunk.copy_from_slice(&v[..chunk.len()]);
            }
        }
    }
}

#[cfg(test)]
mod property_tests {
    /// Property-style round-trip test over randomly generated datasets,
    /// using the deterministic PRNG from the unit-test module.
    use super::tests::Rng;
    use crate::dataset::{parse, serialize, Dataset, Encoding};
    use crate::element::DataElement;
    use crate::tag::Tag;
    use crate::uid::TransferSyntax;
    use crate::vr::Vr;

    const TAGS: [Tag; 4] = [
        Tag::new(0x0010, 0x0010),
        Tag::new(0x0010, 0x0020),
        Tag::new(0x0008, 0x1030),
        Tag::new(0x0020, 0x0013),
    ];

    fn ts_of(enc: Encoding) -> TransferSyntax {
        match (enc.big_endian, enc.explicit_vr) {
            (false, false) => TransferSyntax::ImplicitVrLittleEndian,
            (false, true) => TransferSyntax::ExplicitVrLittleEndian,
            (true, true) => TransferSyntax::ExplicitVrBigEndian,
            _ => unreachable!(),
        }
    }

    #[test]
    fn property_random_datasets_roundtrip() {
        let mut rng = Rng::new(0x0D1C_05E5_u64);
        for iteration in 0..200 {
            let mut ds = Dataset {
                elements: Vec::new(),
                encoding: {
                    let big_endian = rng.below(2) == 0;
                    // (big-endian + implicit) is not a valid DICOM encoding.
                    let explicit_vr = big_endian || rng.below(4) != 0;
                    Encoding { big_endian, explicit_vr }
                },
            };
            for _ in 0..rng.below(8) {
                let tag = TAGS[rng.below(TAGS.len())];
                let len = rng.below(30) + 1;
                let mut buf = vec![0u8; len];
                rng.fill(&mut buf);
                for b in buf.iter_mut() {
                    *b = b'!' + (*b % 90); // printable ASCII
                }
                // Use the dictionary VR so Implicit VR parsing re-infers it.
                let vr = crate::dict::vr_of(tag).unwrap_or(Vr::Un);
                ds.upsert(DataElement::new(tag, vr, buf));
            }
            let bytes = serialize(&ds)
                .unwrap_or_else(|e| panic!("iter {iteration}: serialize failed: {e}"));
            let back = parse(&bytes, ts_of(ds.encoding))
                .unwrap_or_else(|e| panic!("iter {iteration}: parse failed: {e}"));
            assert_eq!(back, ds, "iter {iteration}: model mismatch");
        }
    }

    #[test]
    fn property_never_panics_on_random_bytes() {
        // Fuzz-lite: parsing arbitrary bytes must never panic.
        let mut rng = Rng::new(0xF00D);
        let mut buf = [0u8; 256];
        for _ in 0..500 {
            rng.fill(&mut buf);
            let len = rng.below(buf.len());
            let _ = parse(&buf[..len], TransferSyntax::ExplicitVrLittleEndian);
            let _ = parse(&buf[..len], TransferSyntax::ImplicitVrLittleEndian);
        }
    }
}
