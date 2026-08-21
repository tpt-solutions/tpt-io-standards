//! PS3.10 File Format: 128-byte preamble + `"DICM"` magic + File Meta
//! Information group + the main dataset.

use alloc::string::{String, ToString};
use alloc::vec::Vec;

use crate::dataset;
use crate::error::{refs, DicomError, Result};
use crate::tag::Tag;
use crate::uid::TransferSyntax;

/// The `"DICM"` magic at offset 128 of every Part 10 file.
const DICM: [u8; 4] = *b"DICM";

/// A parsed DICOM Part 10 file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DicomFile {
    /// The 128-byte preamble (application-specific, often zeros).
    pub preamble: Vec<u8>,
    /// File Meta Information group `(0002,xxxx)` elements.
    pub meta_elements: Vec<crate::element::DataElement>,
    /// The main dataset (all non-meta groups).
    pub dataset: dataset::Dataset,
}

impl DicomFile {
    /// Parse a complete Part 10 file from bytes.
    pub fn parse(input: &[u8]) -> Result<Self> {
        if input.len() < 132 || input[128..132] != DICM {
            return Err(DicomError::MissingDicmPrefix { offset: 128 });
        }
        // Meta group is always Explicit VR LE (PS3.10 §7.1).
        let meta_enc = dataset::Encoding { big_endian: false, explicit_vr: true };
        let mut offset = 132usize;
        let mut meta_elements = Vec::new();
        loop {
            if offset + 4 > input.len() {
                return Err(DicomError::Core(
                    tpt_io_core::ParseError::eof(offset).with_spec(refs::PS3_10),
                ));
            }
            let group = u16::from_le_bytes([input[offset], input[offset + 1]]);
            if group != Tag::META_GROUP {
                break; // start of the main dataset
            }
            let elem_num = u16::from_le_bytes([input[offset + 2], input[offset + 3]]);
            let (elem, next) =
                dataset::parse_element(input, offset, input.len(), meta_enc, group, elem_num)?;
            meta_elements.push(elem);
            offset = next;
        }

        // Transfer Syntax UID from the meta group determines dataset encoding.
        let ts_uid = meta_elements
            .iter()
            .find(|e| e.tag == Tag::new(0x0002, 0x0010))
            .and_then(crate::element::DataElement::as_string)
            .map(|s| s.trim().to_string())
            .ok_or_else(|| DicomError::MissingRequiredElement { tag: "(0002,0010)".into() })?;
        let ts = TransferSyntax::from_uid(&ts_uid)
            .ok_or_else(|| DicomError::UnsupportedTransferSyntax(ts_uid.clone()))?;
        let enc = dataset::Encoding::for_transfer_syntax(ts)
            .ok_or_else(|| DicomError::UnsupportedTransferSyntax(ts.uid().into()))?;

        let (elements, _) = dataset::parse_dataset(input, offset, input.len(), enc)?;
        Ok(Self {
            preamble: input[..128].to_vec(),
            meta_elements,
            dataset: dataset::Dataset { elements, encoding: enc },
        })
    }

    /// The transfer syntax this file was encoded with.
    pub fn transfer_syntax(&self) -> Result<TransferSyntax> {
        self.meta_elements
            .iter()
            .find(|e| e.tag == Tag::new(0x0002, 0x0010))
            .and_then(crate::element::DataElement::as_string)
            .and_then(|s| TransferSyntax::from_uid(s.trim()))
            .ok_or_else(|| DicomError::MissingRequiredElement { tag: "(0002,0010)".into() })
    }

    /// Serialize back to a complete Part 10 file.
    ///
    /// Note: the meta group's `(0002,0000)` group length is recomputed.
    pub fn serialize(&self) -> Result<Vec<u8>> {
        self.transfer_syntax()?;
        let mut out = Vec::new();
        out.extend_from_slice(&self.preamble[..128.min(self.preamble.len())]);
        while out.len() < 128 {
            out.push(0);
        }
        out.extend_from_slice(&DICM);

        // Rebuild meta group with correct group length.
        let meta_enc = dataset::Encoding { big_endian: false, explicit_vr: true };
        let mut body = Vec::new();
        for e in &self.meta_elements {
            if e.tag == Tag::new(0x0002, 0x0000) {
                continue;
            }
            dataset::serialize_dataset(core::slice::from_ref(e), meta_enc, &mut body)?;
        }
        let group_len =
            crate::element::DataElement::ul(Tag::new(0x0002, 0x0000), body.len() as u32);
        dataset::serialize_dataset(core::slice::from_ref(&group_len), meta_enc, &mut out)?;
        out.extend_from_slice(&body);

        dataset::serialize_dataset(&self.dataset.elements, self.dataset.encoding, &mut out)?;
        Ok(out)
    }

    /// Convenience accessor: MediaStorageSOPClassUID.
    pub fn media_storage_sop_class(&self) -> Option<String> {
        self.meta_elements
            .iter()
            .find(|e| e.tag == Tag::new(0x0002, 0x0002))
            .and_then(crate::element::DataElement::as_string)
    }

    /// Whether these bytes look like a Part 10 file (preamble + magic).
    pub fn is_part10(input: &[u8]) -> bool {
        input.len() >= 132 && input[128..132] == DICM
    }
}

// Reader trait integration (tpt-io-core pattern).
/// Unit struct implementing [`tpt_io_core::Reader`] for Part 10 files.
#[derive(Debug, Clone, Copy, Default)]
pub struct DicomFileReader;

impl tpt_io_core::Reader<DicomFile> for DicomFileReader {
    type Error = DicomError;

    fn parse(&self, input: &[u8]) -> core::result::Result<DicomFile, Self::Error> {
        DicomFile::parse(input)
    }
}

/// Writer trait integration (tpt-io-core pattern).
#[derive(Debug, Clone, Copy, Default)]
pub struct DicomFileWriter;

impl tpt_io_core::Writer<DicomFile> for DicomFileWriter {
    type Error = DicomError;

    fn serialize(&self, value: &DicomFile) -> core::result::Result<Vec<u8>, Self::Error> {
        value.serialize()
    }
}

/// Syntactic/semantic validation of a parsed file, following the core
/// `Validator` split: structure (magic, meta group, tag ordering) vs.
/// content (UID validity, SOP class consistency).
#[derive(Debug, Clone, Copy, Default)]
pub struct DicomValidator;

impl tpt_io_core::Validator<DicomFile> for DicomValidator {
    type Error = DicomError;

    fn validate_syntactic(&self, value: &DicomFile) -> core::result::Result<(), Self::Error> {
        // Tags must be strictly ascending within each group boundary.
        let mut prev: Option<Tag> = None;
        for e in &value.dataset.elements {
            if let Some(p) = prev {
                if e.tag.group == p.group && e.tag <= p {
                    return Err(DicomError::invalid_value(
                        0,
                        alloc::format!("dataset tags out of ascending order at {e:?}"),
                    ));
                }
            }
            prev = Some(e.tag);
        }
        Ok(())
    }

    fn validate_semantic(&self, value: &DicomFile) -> core::result::Result<(), Self::Error> {
        // All UID-valued elements must satisfy PS3.5 §9.
        for e in &value.dataset.elements {
            if e.vr == crate::vr::Vr::Ui {
                if let Some(s) = e.as_string() {
                    crate::uid::validate_uid(s.trim())
                        .map_err(|reason| DicomError::invalid_value(0, reason))?;
                }
            }
        }
        // Meta SOP class must match dataset SOP class when both are present.
        let meta_class = value.media_storage_sop_class();
        let sop_class =
            value.dataset.string(Tag::new(0x0008, 0x0016)).map(|s| s.trim().to_string());
        if let (Some(m), Some(s)) = (meta_class, sop_class) {
            if m.trim() != s.trim() {
                return Err(DicomError::invalid_value(
                    0,
                    alloc::format!("MediaStorageSOPClassUID {m} does not match SOPClassUID {s}"),
                ));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dataset::{Dataset, Encoding};
    use crate::element::DataElement;
    use crate::vr::Vr;

    fn sample_file(ts: TransferSyntax) -> DicomFile {
        let mut ds =
            Dataset { elements: Vec::new(), encoding: Encoding::for_transfer_syntax(ts).unwrap() };
        ds.upsert(DataElement::text(Tag::new(0x0008, 0x0016), Vr::Ui, "1.2.840.10008.5.1.4.1.1.2"));
        ds.upsert(DataElement::text(Tag::new(0x0008, 0x0018), Vr::Ui, "1.2.3.4.5"));
        ds.upsert(DataElement::text(Tag::new(0x0010, 0x0010), Vr::Pn, "Doe^Jane"));
        let meta = vec![
            DataElement::ul(Tag::new(0x0002, 0x0000), 0), // recomputed on write
            DataElement::ui_text(Tag::new(0x0002, 0x0002), "1.2.840.10008.5.1.4.1.1.2"),
            DataElement::ui_text(Tag::new(0x0002, 0x0003), "1.2.3.4.5"),
            DataElement::ui_text(Tag::new(0x0002, 0x0010), ts.uid()),
            DataElement::ui_text(Tag::new(0x0002, 0x0012), "1.2.826.0.1.3680043.9.7133"),
        ];
        DicomFile { preamble: vec![0u8; 128], meta_elements: meta, dataset: ds }
    }

    impl DataElement {
        fn ui_text(tag: Tag, s: &str) -> Self {
            Self::text(tag, Vr::Ui, s)
        }
    }

    #[test]
    fn part10_roundtrip_explicit_le() {
        let f = sample_file(TransferSyntax::ExplicitVrLittleEndian);
        let bytes = f.serialize().unwrap();
        assert!(DicomFile::is_part10(&bytes));
        assert_eq!(bytes[128..132], *b"DICM");
        let back = DicomFile::parse(&bytes).unwrap();
        assert_eq!(back.transfer_syntax().unwrap(), TransferSyntax::ExplicitVrLittleEndian);
        assert_eq!(back.dataset.string(Tag::new(0x0010, 0x0010)).as_deref(), Some("Doe^Jane"));
        // Group length must be correct.
        let len = back.meta_elements[0].as_u32().unwrap();
        assert!(len > 0);
        // Full byte-level round trip.
        assert_eq!(back.serialize().unwrap(), bytes);
    }

    #[test]
    fn part10_roundtrip_implicit_le() {
        let f = sample_file(TransferSyntax::ImplicitVrLittleEndian);
        let bytes = f.serialize().unwrap();
        let back = DicomFile::parse(&bytes).unwrap();
        assert_eq!(back.transfer_syntax().unwrap(), TransferSyntax::ImplicitVrLittleEndian);
        assert_eq!(back.dataset.string(Tag::new(0x0010, 0x0010)).as_deref(), Some("Doe^Jane"));
    }

    #[test]
    fn missing_magic_rejected() {
        let err = DicomFile::parse(&[0u8; 200]).unwrap_err();
        assert_eq!(err.offset(), Some(128));
        let err2 = DicomFile::parse(&[0u8; 100]).unwrap_err();
        assert_eq!(err2.offset(), Some(128));
    }

    #[test]
    fn validator_split() {
        use tpt_io_core::Validator;
        let v = DicomValidator;
        let good = sample_file(TransferSyntax::ExplicitVrLittleEndian);
        assert!(v.validate_syntactic(&good).is_ok());
        assert!(v.validate_semantic(&good).is_ok());
        // Break tag order.
        let mut bad = good.clone();
        bad.dataset.elements.reverse();
        assert!(v.validate_syntactic(&bad).is_err());
        // Mismatched SOP classes.
        let mut mismatch = sample_file(TransferSyntax::ExplicitVrLittleEndian);
        for e in mismatch.meta_elements.iter_mut() {
            if e.tag == Tag::new(0x0002, 0x0002) {
                *e = DataElement::ui_text(Tag::new(0x0002, 0x0002), "1.2.840.10008.5.1.4.1.1.4");
            }
        }
        assert!(v.validate_semantic(&mismatch).is_err());
    }
}
