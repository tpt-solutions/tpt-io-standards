//! DICOM parser: explicit and implicit VR transfer syntax parsing (PS3.5 §7.1).
//!
//! The parser reads the 128-byte preamble + "DICM" magic, then walks through
//! the file meta-information group (0002,xxxx) and the main data set, decoding
//! each element according to the detected transfer syntax.

use tpt_io_core::ParseError;

use crate::error::DicomError;
use crate::model::{Dataset, DicomFile, Element, Tag, Value, VR};
use crate::tag::{lookup_tag, VRCode};
use crate::uid::TransferSyntax;

/// DICOM file preamble size in bytes.
pub const PREAMBLE_SIZE: usize = 128;

/// DICOM magic bytes: "DICM".
pub const DICM_MAGIC: [u8; 4] = *b"DICM";

/// A DICOM reader implementing [`tpt_io_core::Reader`].
#[derive(Debug, Clone, Default)]
pub struct DicomReader {
    /// Whether to require the 128-byte preamble + "DICM" magic.
    require_preamble: bool,
}

impl DicomReader {
    /// Create a new reader that requires the standard preamble.
    pub fn new() -> Self {
        Self { require_preamble: true }
    }

    /// Set whether to require the preamble + "DICM" magic (default: true).
    pub fn with_preamble_required(mut self, required: bool) -> Self {
        self.require_preamble = required;
        self
    }

    /// Parse a DICOM file from raw bytes.
    pub fn parse_bytes(&self, input: &[u8]) -> Result<DicomFile, DicomError> {
        parse_dicom(input, self.require_preamble)
    }
}

impl tpt_io_core::Reader<DicomFile> for DicomReader {
    type Error = DicomError;

    fn parse(&self, input: &[u8]) -> Result<DicomFile, DicomError> {
        self.parse_bytes(input)
    }
}

/// Parse a DICOM file from raw bytes.
pub fn parse_dicom(input: &[u8], require_preamble: bool) -> Result<DicomFile, DicomError> {
    let mut offset = 0;

    // Step 1: preamble + magic (optional but standard).
    if require_preamble {
        if input.len() < PREAMBLE_SIZE + DICM_MAGIC.len() {
            return Err(DicomError::Core(ParseError::eof(input.len())));
        }
        if input[PREAMBLE_SIZE..PREAMBLE_SIZE + 4] != DICM_MAGIC {
            return Err(DicomError::Core(ParseError::custom(0, "missing DICM magic")));
        }
        offset = PREAMBLE_SIZE + 4;
    }

    // Step 2: file meta-information group (0002,xxxx) — always explicit VR little-endian.
    let (meta, bytes_read) = parse_dataset(input, offset, true, true)?;
    offset += bytes_read;

    // Step 3: detect transfer syntax from meta group.
    let ts_uid = meta.string(Tag::new(0x0002, 0x0010));
    let transfer_syntax = match ts_uid {
        Some(uid) => TransferSyntax::from_uid(uid).unwrap_or(TransferSyntax::ImplicitVrLittleEndian),
        None => TransferSyntax::ImplicitVrLittleEndian,
    };

    // Step 4: main data set using detected transfer syntax.
    let (dataset, _bytes_read) =
        parse_dataset(input, offset, transfer_syntax.is_explicit_vr(), transfer_syntax.is_little_endian())?;

    Ok(DicomFile { meta, dataset, transfer_syntax })
}

/// Parse a dataset (sequence of elements) from `input` starting at `offset`.
///
/// `explicit_vr` controls whether VR is read for each element.
/// `little_endian` controls byte ordering.
pub fn parse_dataset(
    input: &[u8],
    offset: usize,
    explicit_vr: bool,
    little_endian: bool,
) -> Result<(Dataset, usize), DicomError> {
    let mut dataset = Dataset::new();
    let mut pos = offset;

    while pos + 4 <= input.len() {
        let (element, bytes_read) = parse_element(input, pos, explicit_vr, little_endian)?;
        dataset.push(element);
        pos += bytes_read;
    }

    Ok((dataset, pos - offset))
}

/// Parse a single element from `input` starting at `offset`.
///
/// Returns the element and the number of bytes consumed.
pub fn parse_element(
    input: &[u8],
    offset: usize,
    explicit_vr: bool,
    little_endian: bool,
) -> Result<(Element, usize), DicomError> {
    let mut pos = offset;

    // Read tag: group (u16) + element (u16).
    let group = read_u16(input, pos, little_endian)?;
    pos += 2;
    let element_num = read_u16(input, pos, little_endian)?;
    pos += 2;
    let tag = Tag::new(group, element_num);

    let (vr, value, header_size) = if explicit_vr {
        parse_explicit_vr_element(input, pos, little_endian)?
    } else {
        parse_implicit_vr_element(input, pos, little_endian, group, element_num)?
    };
    pos += header_size;

    // Skip value bytes.
    let value_len = value_byte_length(&value);
    pos += value_len;

    Ok((Element { tag, vr, value }, pos - offset))
}

/// Parse an element using explicit VR transfer syntax.
fn parse_explicit_vr_element(
    input: &[u8],
    offset: usize,
    little_endian: bool,
) -> Result<(VR, Value, usize), DicomError> {
    let mut pos = offset;

    // Read VR (2 ASCII bytes).
    if pos + 2 > input.len() {
        return Err(DicomError::Core(ParseError::eof(input.len())));
    }
    let vr_bytes = [input[pos], input[pos + 1]];
    pos += 2;

    let vr = match VRCode::from_bytes(vr_bytes) {
        Some(code) => VR::Known(code),
        None => VR::Unknown(vr_bytes),
    };

    // Determine length field size based on VR.
    let (value_len, length_field_size) = match vr {
        VR::Known(VRCode::OB | VRCode::OD | VRCode::OF | VRCode::OL | VRCode::OV | VRCode::OW | VRCode::SQ | VRCode::UC | VRCode::UN | VRCode::UR | VRCode::UT) => {
            // These VRs have 2 reserved bytes + 4-byte length.
            if pos + 6 > input.len() {
                return Err(DicomError::Core(ParseError::eof(input.len())));
            }
            let length = u32::from_le_bytes([input[pos + 2], input[pos + 3], input[pos + 4], input[pos + 5]]) as usize;
            (length, 6)
        }
        _ => {
            // Standard 2-byte length field.
            if pos + 2 > input.len() {
                return Err(DicomError::Core(ParseError::eof(input.len())));
            }
            let length = if little_endian {
                u16::from_le_bytes([input[pos], input[pos + 1]])
            } else {
                u16::from_be_bytes([input[pos], input[pos + 1]])
            } as usize;
            (length, 2)
        }
    };
    pos += length_field_size;

    let header_consumed = 2 + length_field_size; // VR(2) + length field

    let value = decode_value(input, pos, value_len, &vr, little_endian, true)?;

    Ok((vr, value, header_consumed))
}

/// Parse an element using implicit VR transfer syntax.
fn parse_implicit_vr_element(
    input: &[u8],
    offset: usize,
    little_endian: bool,
    group: u16,
    element_num: u16,
) -> Result<(VR, Value, usize), DicomError> {
    let mut pos = offset;

    // 4-byte length field (no VR).
    if pos + 4 > input.len() {
        return Err(DicomError::Core(ParseError::eof(input.len())));
    }
    let length = if little_endian {
        u32::from_le_bytes([input[pos], input[pos + 1], input[pos + 2], input[pos + 3]])
    } else {
        u32::from_be_bytes([input[pos], input[pos + 1], input[pos + 2], input[pos + 3]])
    } as usize;
    pos += 4;

    // Look up VR from tag dictionary.
    let vr = match lookup_tag(group, element_num) {
        Some(info) => {
            if info.vr.len() == 1 {
                VR::Known(info.vr[0])
            } else {
                VR::Known(info.vr[0]) // Use first VR as default
            }
        }
        None => VR::Unknown(*b"UN"),
    };

    let value = decode_value(input, pos, length, &vr, little_endian, false)?;

    Ok((vr, value, 4))
}

/// Decode a value from raw bytes based on VR.
fn decode_value(
    input: &[u8],
    offset: usize,
    length: usize,
    vr: &VR,
    little_endian: bool,
    explicit: bool,
) -> Result<Value, DicomError> {
    if length == 0 {
        return Ok(Value::Raw(Vec::new()));
    }

    if offset + length > input.len() {
        return Err(DicomError::Core(ParseError::eof(input.len())));
    }

    let bytes = &input[offset..offset + length];

    match vr {
        VR::Known(VRCode::SQ) => {
            let items = parse_sequence(bytes, length, little_endian)?;
            Ok(Value::Sequence(items))
        }
        VR::Known(v) if is_string_vr(*v) => {
            Ok(Value::String(decode_dicom_string(bytes)))
        }
        VR::Known(VRCode::US) => {
            if length >= 2 {
                let val = if little_endian {
                    u16::from_le_bytes([bytes[0], bytes[1]])
                } else {
                    u16::from_be_bytes([bytes[0], bytes[1]])
                };
                Ok(Value::U16(val))
            } else {
                Ok(Value::Raw(bytes.to_vec()))
            }
        }
        VR::Known(VRCode::SS) => {
            if length >= 2 {
                let val = if little_endian {
                    i16::from_le_bytes([bytes[0], bytes[1]])
                } else {
                    i16::from_be_bytes([bytes[0], bytes[1]])
                };
                Ok(Value::I16(val))
            } else {
                Ok(Value::Raw(bytes.to_vec()))
            }
        }
        VR::Known(VRCode::UL) => {
            if length >= 4 {
                let val = if little_endian {
                    u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
                } else {
                    u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
                };
                Ok(Value::U32(val))
            } else {
                Ok(Value::Raw(bytes.to_vec()))
            }
        }
        VR::Known(VRCode::SL) => {
            if length >= 4 {
                let val = if little_endian {
                    i32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
                } else {
                    i32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
                };
                Ok(Value::I32(val))
            } else {
                Ok(Value::Raw(bytes.to_vec()))
            }
        }
        VR::Known(VRCode::FL) => {
            if length >= 4 {
                let val = if little_endian {
                    f32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
                } else {
                    f32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
                };
                Ok(Value::F32(val))
            } else {
                Ok(Value::Raw(bytes.to_vec()))
            }
        }
        VR::Known(VRCode::FD) => {
            if length >= 8 {
                let val = if little_endian {
                    f64::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7]])
                } else {
                    f64::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7]])
                };
                Ok(Value::F64(val))
            } else {
                Ok(Value::Raw(bytes.to_vec()))
            }
        }
        VR::Known(VRCode::AT) => {
            if length >= 4 {
                let group = if little_endian {
                    u16::from_le_bytes([bytes[0], bytes[1]])
                } else {
                    u16::from_be_bytes([bytes[0], bytes[1]])
                };
                let element = if little_endian {
                    u16::from_le_bytes([bytes[2], bytes[3]])
                } else {
                    u16::from_be_bytes([bytes[2], bytes[3]])
                };
                Ok(Value::String(alloc::format!("({group:04X},{element:04X})")))
            } else {
                Ok(Value::Raw(bytes.to_vec()))
            }
        }
        _ => {
            // For OB, OW, OD, OF, OL, OV, UN, Unknown — return raw bytes.
            Ok(Value::Raw(bytes.to_vec()))
        }
    }
}

/// Parse a sequence of items from the input.
fn parse_sequence(
    input: &[u8],
    length: usize,
    little_endian: bool,
) -> Result<Vec<Dataset>, DicomError> {
    let mut items = Vec::new();
    let end = if length == 0xFFFF_FFFF || length >= input.len() {
        input.len()
    } else {
        length
    };

    let mut pos = 0;
    while pos + 8 <= core::cmp::min(end, input.len()) {
        let item_group = if little_endian {
            u16::from_le_bytes([input[pos], input[pos + 1]])
        } else {
            u16::from_be_bytes([input[pos], input[pos + 1]])
        };
        let item_element = if little_endian {
            u16::from_le_bytes([input[pos + 2], input[pos + 3]])
        } else {
            u16::from_be_bytes([input[pos + 2], input[pos + 3]])
        };
        let item_length = if little_endian {
            u32::from_le_bytes([input[pos + 4], input[pos + 5], input[pos + 6], input[pos + 7]])
        } else {
            u32::from_be_bytes([input[pos + 4], input[pos + 5], input[pos + 6], input[pos + 7]])
        } as usize;
        pos += 8;

        if item_group == 0xFFFE && item_element == 0xE0DD {
            // Sequence Delimitation Item.
            break;
        }
        if item_group == 0xFFFE && item_element == 0xE00D {
            // Item Delimitation Item.
            break;
        }

        if item_group != 0xFFFE || item_element != 0xE000 {
            return Err(DicomError::InvalidTag { offset: pos - 8, group: item_group, element: item_element });
        }

        let item_end = core::cmp::min(pos + item_length, input.len());
        let (item_dataset, _item_read) =
            parse_dataset(input, pos, true, little_endian)?;
        items.push(item_dataset);
        pos = item_end;
    }

    Ok(items)
}

/// Determine whether a VR represents a string-type value.
fn is_string_vr(vr: VRCode) -> bool {
    matches!(
        vr,
        VRCode::AE | VRCode::AS | VRCode::CS | VRCode::DA | VRCode::DS | VRCode::DT
            | VRCode::IS | VRCode::LO | VRCode::LT | VRCode::PN | VRCode::SH | VRCode::ST
            | VRCode::TM | VRCode::UC | VRCode::UI | VRCode::UR | VRCode::UT
    )
}

/// Decode a DICOM string, trimming trailing whitespace and null bytes.
fn decode_dicom_string(bytes: &[u8]) -> String {
    let trimmed = bytes
        .iter()
        .rposition(|&b| !matches!(b, b' ' | b'\0'))
        .map(|p| &bytes[..=p])
        .unwrap_or(&[]);
    String::from_utf8_lossy(trimmed).into_owned()
}

/// Return the byte length of the value in the element.
fn value_byte_length(value: &Value) -> usize {
    match value {
        Value::Raw(b) => b.len(),
        Value::String(s) => s.len(),
        Value::I32(_) => 4,
        Value::U32(_) => 4,
        Value::I16(_) => 2,
        Value::U16(_) => 2,
        Value::F32(_) => 4,
        Value::F64(_) => 8,
        Value::Sequence(_) => 0,
    }
}

// --- Low-level byte reading helpers ---

fn read_u16(input: &[u8], offset: usize, little_endian: bool) -> Result<u16, DicomError> {
    if offset + 2 > input.len() {
        return Err(DicomError::Core(ParseError::eof(input.len())));
    }
    Ok(if little_endian {
        u16::from_le_bytes([input[offset], input[offset + 1]])
    } else {
        u16::from_be_bytes([input[offset], input[offset + 1]])
    })
}

fn read_u32(input: &[u8], offset: usize, little_endian: bool) -> Result<u32, DicomError> {
    if offset + 4 > input.len() {
        return Err(DicomError::Core(ParseError::eof(input.len())));
    }
    Ok(if little_endian {
        u32::from_le_bytes([input[offset], input[offset + 1], input[offset + 2], input[offset + 3]])
    } else {
        u32::from_be_bytes([input[offset], input[offset + 1], input[offset + 2], input[offset + 3]])
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_explicit_vr_element() {
        // Construct: (0008,0016) UI 1.2.840.10008.5.1.4.1.1.2\0
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&0x0008u16.to_le_bytes()); // group
        bytes.extend_from_slice(&0x0016u16.to_le_bytes()); // element
        bytes.extend_from_slice(b"UI"); // VR
        let uid = b"1.2.840.10008.5.1.4.1.1.2\0";
        bytes.extend_from_slice(&(uid.len() as u16).to_le_bytes()); // length
        bytes.extend_from_slice(uid); // value

        let (elem, read) = parse_element(&bytes, 0, true, true).unwrap();
        assert_eq!(elem.tag, Tag::new(0x0008, 0x0016));
        assert_eq!(elem.vr, VR::Known(VRCode::UI));
        assert_eq!(read, 4 + 2 + 2 + uid.len()); // tag(4) + vr(2) + length(2) + value
    }

    #[test]
    fn parse_full_dicom() {
        // Build a minimal DICOM file.
        let mut dcm = Vec::new();
        // Preamble: 128 zero bytes.
        dcm.extend_from_slice(&[0u8; 128]);
        // Magic: "DICM"
        dcm.extend_from_slice(b"DICM");

        // File meta-information group: Transfer Syntax UID (0002,0010)
        // Explicit VR Little Endian.
        dcm.extend_from_slice(&0x0002u16.to_le_bytes()); // group
        dcm.extend_from_slice(&0x0010u16.to_le_bytes()); // element
        dcm.extend_from_slice(b"UI"); // VR
        let ts = b"1.2.840.10008.1.2.1\0";
        dcm.extend_from_slice(&(ts.len() as u16).to_le_bytes()); // length
        dcm.extend_from_slice(ts); // value

        // Main dataset: SOP Class UID (0008,0016)
        dcm.extend_from_slice(&0x0008u16.to_le_bytes()); // group
        dcm.extend_from_slice(&0x0016u16.to_le_bytes()); // element
        dcm.extend_from_slice(b"UI"); // VR
        let sop = b"1.2.840.10008.5.1.4.1.1.2\0";
        dcm.extend_from_slice(&(sop.len() as u16).to_le_bytes()); // length
        dcm.extend_from_slice(sop); // value

        let reader = DicomReader::new();
        let file = reader.parse_bytes(&dcm).unwrap();
        assert_eq!(file.transfer_syntax, TransferSyntax::ExplicitVrLittleEndian);
        assert!(file.meta.string(Tag::new(0x0002, 0x0010)).is_some());
        assert!(file.dataset.string(Tag::new(0x0008, 0x0016)).is_some());
    }

    #[test]
    fn parse_implicit_vr_dicom() {
        // Build a minimal DICOM file with implicit VR.
        let mut dcm = Vec::new();
        dcm.extend_from_slice(&[0u8; 128]);
        dcm.extend_from_slice(b"DICM");

        // File meta-information: Transfer Syntax UID (0002,0010) — explicit VR.
        dcm.extend_from_slice(&0x0002u16.to_le_bytes());
        dcm.extend_from_slice(&0x0010u16.to_le_bytes());
        dcm.extend_from_slice(b"UI");
        let ts = b"1.2.840.10008.1.2\0"; // Implicit VR Little Endian
        dcm.extend_from_slice(&(ts.len() as u16).to_le_bytes());
        dcm.extend_from_slice(ts);

        // Main dataset: SOP Class UID (0008,0016) — implicit VR.
        dcm.extend_from_slice(&0x0008u16.to_le_bytes());
        dcm.extend_from_slice(&0x0016u16.to_le_bytes());
        let sop = b"1.2.840.10008.5.1.4.1.1.2\0";
        dcm.extend_from_slice(&(sop.len() as u32).to_le_bytes()); // 4-byte length
        dcm.extend_from_slice(sop);

        let reader = DicomReader::new();
        let file = reader.parse_bytes(&dcm).unwrap();
        assert_eq!(file.transfer_syntax, TransferSyntax::ImplicitVrLittleEndian);
        assert!(file.dataset.string(Tag::new(0x0008, 0x0016)).is_some());
    }
}
