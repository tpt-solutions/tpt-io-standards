//! DICOM writer: serialize datasets back to bytes (PS3.5 §7.1).

use alloc::vec::Vec;

use crate::error::DicomError;
use crate::model::{Dataset, DicomFile, Element, Value, VR};
use crate::tag::VRCode;
use crate::uid::TransferSyntax;

/// A DICOM writer implementing [`tpt_io_core::Writer`].
#[derive(Debug, Clone, Default)]
pub struct DicomWriter {
    /// The transfer syntax to use for encoding.
    transfer_syntax: TransferSyntax,
}

impl DicomWriter {
    /// Create a new writer using explicit VR little-endian.
    pub fn new() -> Self {
        Self { transfer_syntax: TransferSyntax::ExplicitVrLittleEndian }
    }

    /// Set the transfer syntax for encoding.
    pub fn with_transfer_syntax(mut self, ts: TransferSyntax) -> Self {
        self.transfer_syntax = ts;
        self
    }

    /// Serialize a DICOM file to bytes.
    pub fn serialize_file(&self, file: &DicomFile) -> Result<Vec<u8>, DicomError> {
        let mut out = Vec::new();

        // Preamble + magic.
        out.extend_from_slice(&[0u8; 128]);
        out.extend_from_slice(b"DICM");

        // File meta-information group (always explicit VR LE).
        serialize_dataset(&mut out, &file.meta, true, true)?;

        // Main dataset (per transfer syntax).
        serialize_dataset(
            &mut out,
            &file.dataset,
            file.transfer_syntax.is_explicit_vr(),
            file.transfer_syntax.is_little_endian(),
        )?;

        Ok(out)
    }

    /// Serialize a dataset to bytes.
    pub fn serialize_data(&self, dataset: &Dataset) -> Result<Vec<u8>, DicomError> {
        let mut out = Vec::new();
        serialize_dataset(
            &mut out,
            dataset,
            self.transfer_syntax.is_explicit_vr(),
            self.transfer_syntax.is_little_endian(),
        )?;
        Ok(out)
    }
}

impl tpt_io_core::Writer<Dataset> for DicomWriter {
    type Error = DicomError;

    fn serialize(&self, value: &Dataset) -> Result<Vec<u8>, DicomError> {
        self.serialize_data(value)
    }
}

/// Serialize a dataset into `out`.
fn serialize_dataset(
    out: &mut Vec<u8>,
    dataset: &Dataset,
    explicit_vr: bool,
    little_endian: bool,
) -> Result<(), DicomError> {
    for element in &dataset.elements {
        serialize_element(out, element, explicit_vr, little_endian)?;
    }
    Ok(())
}

/// Serialize a single element into `out`.
fn serialize_element(
    out: &mut Vec<u8>,
    element: &Element,
    explicit_vr: bool,
    little_endian: bool,
) -> Result<(), DicomError> {
    // Tag.
    if little_endian {
        out.extend_from_slice(&element.tag.group().to_le_bytes());
        out.extend_from_slice(&element.tag.element().to_le_bytes());
    } else {
        out.extend_from_slice(&element.tag.group().to_be_bytes());
        out.extend_from_slice(&element.tag.element().to_be_bytes());
    }

    if explicit_vr {
        serialize_element_explicit_vr(out, element, little_endian)
    } else {
        serialize_element_implicit_vr(out, element, little_endian)
    }
}

fn serialize_element_explicit_vr(
    out: &mut Vec<u8>,
    element: &Element,
    little_endian: bool,
) -> Result<(), DicomError> {
    // VR.
    match &element.vr {
        VR::Known(code) => {
            out.extend_from_slice(&code.as_bytes());
        }
        VR::Unknown(bytes) => {
            out.extend_from_slice(bytes);
        }
    }

    // Value bytes first so we know the length.
    let value_bytes = encode_value(&element.value)?;

    // Length field.
    let vr_is_special = matches!(
        element.vr,
        VR::Known(
            VRCode::OB
                | VRCode::OD
                | VRCode::OF
                | VRCode::OL
                | VRCode::OV
                | VRCode::OW
                | VRCode::SQ
                | VRCode::UC
                | VRCode::UN
                | VRCode::UR
                | VRCode::UT
        )
    );

    if vr_is_special {
        out.extend_from_slice(&[0u8; 2]); // reserved
        if little_endian {
            out.extend_from_slice(&(value_bytes.len() as u32).to_le_bytes());
        } else {
            out.extend_from_slice(&(value_bytes.len() as u32).to_be_bytes());
        }
    } else {
        if little_endian {
            out.extend_from_slice(&(value_bytes.len() as u16).to_le_bytes());
        } else {
            out.extend_from_slice(&(value_bytes.len() as u16).to_be_bytes());
        }
    }

    // Value.
    out.extend_from_slice(&value_bytes);
    Ok(())
}

fn serialize_element_implicit_vr(
    out: &mut Vec<u8>,
    element: &Element,
    little_endian: bool,
) -> Result<(), DicomError> {
    let value_bytes = encode_value(&element.value)?;

    if little_endian {
        out.extend_from_slice(&(value_bytes.len() as u32).to_le_bytes());
    } else {
        out.extend_from_slice(&(value_bytes.len() as u32).to_be_bytes());
    }

    out.extend_from_slice(&value_bytes);
    Ok(())
}

/// Encode a value into bytes.
fn encode_value(value: &Value) -> Result<Vec<u8>, DicomError> {
    match value {
        Value::Raw(bytes) => Ok(bytes.clone()),
        Value::String(s) => Ok(s.as_bytes().to_vec()),
        Value::I32(v) => Ok(v.to_le_bytes().to_vec()),
        Value::U32(v) => Ok(v.to_le_bytes().to_vec()),
        Value::I16(v) => Ok(v.to_le_bytes().to_vec()),
        Value::U16(v) => Ok(v.to_le_bytes().to_vec()),
        Value::F32(v) => Ok(v.to_le_bytes().to_vec()),
        Value::F64(v) => Ok(v.to_le_bytes().to_vec()),
        Value::Sequence(items) => {
            let mut out = Vec::new();
            for item in items {
                // Item tag (FFFE,E000).
                out.extend_from_slice(&0xFFFEu16.to_le_bytes());
                out.extend_from_slice(&0xE000u16.to_le_bytes());
                // Item length (placeholder; we'll compute).
                let item_start = out.len();
                out.extend_from_slice(&0u32.to_le_bytes()); // placeholder
                let item_data_start = out.len();
                serialize_dataset(&mut out, item, true, true)?;
                let item_len = out.len() - item_data_start;
                // Write item length.
                out[item_start..item_start + 4].copy_from_slice(&(item_len as u32).to_le_bytes());
            }
            // Sequence Delimitation Item (FFFE,E0DD) + 0 length.
            out.extend_from_slice(&0xFFFEu16.to_le_bytes());
            out.extend_from_slice(&0xE0DDu16.to_le_bytes());
            out.extend_from_slice(&0u32.to_le_bytes());
            Ok(out)
        }
    }
}

/// Round-trip a dataset through serialization and parsing.
pub fn round_trip_dataset(
    dataset: &Dataset,
    transfer_syntax: TransferSyntax,
) -> Result<Dataset, DicomError> {
    let writer = DicomWriter::new().with_transfer_syntax(transfer_syntax);
    let bytes = writer.serialize_data(dataset)?;
    let (parsed, _) = crate::parser::parse_dataset(
        &bytes,
        0,
        transfer_syntax.is_explicit_vr(),
        transfer_syntax.is_little_endian(),
        None,
    )?;
    Ok(parsed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Element, Tag, VR};
    use crate::DicomReader;

    #[test]
    fn round_trip_string_element() {
        let mut ds = Dataset::new();
        ds.push(Element {
            tag: Tag::new(0x0008, 0x0016),
            vr: VR::Known(VRCode::UI),
            value: Value::String("1.2.840.10008.5.1.4.1.1.2".into()),
        });

        let rt = round_trip_dataset(&ds, TransferSyntax::ExplicitVrLittleEndian).unwrap();
        assert_eq!(rt.string(Tag::new(0x0008, 0x0016)), Some("1.2.840.10008.5.1.4.1.1.2"));
    }

    #[test]
    fn serialize_full_file() {
        let mut meta = Dataset::new();
        meta.push(Element {
            tag: Tag::new(0x0002, 0x0010),
            vr: VR::Known(VRCode::UI),
            value: Value::String("1.2.840.10008.1.2.1".into()),
        });

        let mut dataset = Dataset::new();
        dataset.push(Element {
            tag: Tag::new(0x0008, 0x0016),
            vr: VR::Known(VRCode::UI),
            value: Value::String("1.2.840.10008.5.1.4.1.1.2".into()),
        });

        let file =
            DicomFile { meta, dataset, transfer_syntax: TransferSyntax::ExplicitVrLittleEndian };
        let writer = DicomWriter::new();
        let bytes = writer.serialize_file(&file).unwrap();

        // Parse it back.
        let reader = DicomReader::new();
        let parsed = reader.parse_bytes(&bytes).unwrap();
        assert_eq!(parsed.transfer_syntax, TransferSyntax::ExplicitVrLittleEndian);
        assert_eq!(
            parsed.dataset.string(Tag::new(0x0008, 0x0016)),
            Some("1.2.840.10008.5.1.4.1.1.2")
        );
    }
}
