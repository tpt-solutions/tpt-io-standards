//! Integration tests for DICOM parsing, round-trip, and utilities.

use tpt_io_core::Reader;
use tpt_io_dicom::anonymize::{anonymize_dataset, AnonymizationProfile};
use tpt_io_dicom::hierarchy::HierarchyNavigator;
use tpt_io_dicom::model::{Dataset, DicomFile, Element, Tag, Value, VR};
use tpt_io_dicom::parser::{parse_dataset, parse_dicom, DicomReader};
use tpt_io_dicom::pixel::extract_pixel_descriptor;
use tpt_io_dicom::tag::VRCode;
use tpt_io_dicom::uid::TransferSyntax;
use tpt_io_dicom::writer::{round_trip_dataset, DicomWriter};

/// Build a minimal valid DICOM file in memory.
fn build_minimal_dicom() -> Vec<u8> {
    let mut dcm = vec![0u8; 128];
    dcm.extend_from_slice(b"DICM");

    // File meta: Transfer Syntax UID (0002,0010) — explicit VR LE.
    dcm.extend_from_slice(&0x0002u16.to_le_bytes());
    dcm.extend_from_slice(&0x0010u16.to_le_bytes());
    dcm.extend_from_slice(b"UI");
    let ts = b"1.2.840.10008.1.2.1\0";
    dcm.extend_from_slice(&(ts.len() as u16).to_le_bytes());
    dcm.extend_from_slice(ts);

    // Dataset: SOP Class UID (0008,0016) + SOP Instance UID (0008,0018).
    dcm.extend_from_slice(&0x0008u16.to_le_bytes());
    dcm.extend_from_slice(&0x0016u16.to_le_bytes());
    dcm.extend_from_slice(b"UI");
    let sop = b"1.2.840.10008.5.1.4.1.1.2\0";
    dcm.extend_from_slice(&(sop.len() as u16).to_le_bytes());
    dcm.extend_from_slice(sop);

    dcm.extend_from_slice(&0x0008u16.to_le_bytes());
    dcm.extend_from_slice(&0x0018u16.to_le_bytes());
    dcm.extend_from_slice(b"UI");
    let inst = b"1.2.3.4.5.6.7\0";
    dcm.extend_from_slice(&(inst.len() as u16).to_le_bytes());
    dcm.extend_from_slice(inst);

    dcm
}

#[test]
fn parse_minimal_dicom() {
    let dcm = build_minimal_dicom();
    let file = DicomReader::new().parse(&dcm).unwrap();
    assert_eq!(file.transfer_syntax, TransferSyntax::ExplicitVrLittleEndian);
    assert_eq!(
        file.meta.string(Tag::new(0x0002, 0x0010)),
        Some("1.2.840.10008.1.2.1")
    );
    assert_eq!(
        file.dataset.string(Tag::new(0x0008, 0x0016)),
        Some("1.2.840.10008.5.1.4.1.1.2")
    );
    assert_eq!(
        file.dataset.string(Tag::new(0x0008, 0x0018)),
        Some("1.2.3.4.5.6.7")
    );
}

#[test]
fn parse_without_preamble() {
    // Build a dataset directly (no preamble).
    let mut data = Vec::new();
    data.extend_from_slice(&0x0008u16.to_le_bytes());
    data.extend_from_slice(&0x0016u16.to_le_bytes());
    data.extend_from_slice(b"UI");
    let sop = b"1.2.840.10008.5.1.4.1.1.2\0";
    data.extend_from_slice(&(sop.len() as u16).to_le_bytes());
    data.extend_from_slice(sop);

    let file = parse_dicom(&data, false).unwrap();
    assert_eq!(
        file.dataset.string(Tag::new(0x0008, 0x0016)),
        Some("1.2.840.10008.5.1.4.1.1.2")
    );
}

#[test]
fn round_trip_explicit_vr() {
    let dcm = build_minimal_dicom();
    let file = DicomReader::new().parse(&dcm).unwrap();

    let writer = DicomWriter::new();
    let bytes = writer.serialize_file(&file).unwrap();

    let file2 = DicomReader::new().parse(&bytes).unwrap();
    assert_eq!(file2.transfer_syntax, file.transfer_syntax);
    assert_eq!(
        file2.dataset.string(Tag::new(0x0008, 0x0016)),
        file.dataset.string(Tag::new(0x0008, 0x0016))
    );
}

#[test]
fn round_trip_dataset() {
    let mut ds = Dataset::new();
    ds.push(Element {
        tag: Tag::new(0x0010, 0x0010),
        vr: VR::Known(VRCode::PN),
        value: Value::String("Doe^John".into()),
    });
    ds.push(Element {
        tag: Tag::new(0x0008, 0x0016),
        vr: VR::Known(VRCode::UI),
        value: Value::String("1.2.840.10008.5.1.4.1.1.2".into()),
    });
    ds.push(Element {
        tag: Tag::new(0x0028, 0x0010),
        vr: VR::Known(VRCode::US),
        value: Value::U16(256),
    });

    let rt = round_trip_dataset(&ds, TransferSyntax::ExplicitVrLittleEndian).unwrap();
    assert_eq!(rt.string(Tag::new(0x0010, 0x0010)), Some("Doe^John"));
    assert_eq!(rt.string(Tag::new(0x0008, 0x0016)), Some("1.2.840.10008.5.1.4.1.1.2"));
    assert_eq!(
        rt.value(Tag::new(0x0028, 0x0010)),
        Some(&Value::U16(256))
    );
}

#[test]
fn implicit_vr_round_trip() {
    let mut dcm = vec![0u8; 128];
    dcm.extend_from_slice(b"DICM");

    // Meta: Transfer Syntax UID = Implicit VR Little Endian.
    dcm.extend_from_slice(&0x0002u16.to_le_bytes());
    dcm.extend_from_slice(&0x0010u16.to_le_bytes());
    dcm.extend_from_slice(b"UI");
    let ts = b"1.2.840.10008.1.2\0";
    dcm.extend_from_slice(&(ts.len() as u16).to_le_bytes());
    dcm.extend_from_slice(ts);

    // Dataset: SOP Class UID (implicit VR — 4-byte length).
    dcm.extend_from_slice(&0x0008u16.to_le_bytes());
    dcm.extend_from_slice(&0x0016u16.to_le_bytes());
    let sop = b"1.2.840.10008.5.1.4.1.1.2\0";
    dcm.extend_from_slice(&(sop.len() as u32).to_le_bytes());
    dcm.extend_from_slice(sop);

    let file = DicomReader::new().parse(&dcm).unwrap();
    assert_eq!(file.transfer_syntax, TransferSyntax::ImplicitVrLittleEndian);
    assert_eq!(
        file.dataset.string(Tag::new(0x0008, 0x0016)),
        Some("1.2.840.10008.5.1.4.1.1.2")
    );
}

#[test]
fn pixel_descriptor_extraction() {
    let mut ds = Dataset::new();
    ds.push(Element {
        tag: Tag::new(0x0028, 0x0010),
        vr: VR::Known(VRCode::US),
        value: Value::U16(512),
    });
    ds.push(Element {
        tag: Tag::new(0x0028, 0x0011),
        vr: VR::Known(VRCode::US),
        value: Value::U16(512),
    });
    ds.push(Element {
        tag: Tag::new(0x0028, 0x0100),
        vr: VR::Known(VRCode::US),
        value: Value::U16(16),
    });
    ds.push(Element {
        tag: Tag::new(0x0028, 0x0002),
        vr: VR::Known(VRCode::US),
        value: Value::U16(1),
    });
    ds.push(Element {
        tag: Tag::new(0x0028, 0x0008),
        vr: VR::Known(VRCode::IS),
        value: Value::String("10".into()),
    });

    let desc = extract_pixel_descriptor(&ds).unwrap();
    assert_eq!(desc.rows, 512);
    assert_eq!(desc.columns, 512);
    assert_eq!(desc.bits_allocated, 16);
    assert_eq!(desc.number_of_frames, 10);
    assert_eq!(desc.pixels_per_frame(), 512 * 512);
}

#[test]
fn hierarchy_navigation() {
    let mut ds = Dataset::new();
    ds.push(Element {
        tag: Tag::new(0x0020, 0x000D),
        vr: VR::Known(VRCode::UI),
        value: Value::String("1.2.3.4.5".into()),
    });
    ds.push(Element {
        tag: Tag::new(0x0020, 0x000E),
        vr: VR::Known(VRCode::UI),
        value: Value::String("1.2.3.4.5.6".into()),
    });
    ds.push(Element {
        tag: Tag::new(0x0008, 0x0018),
        vr: VR::Known(VRCode::UI),
        value: Value::String("1.2.3.4.5.6.7".into()),
    });

    let nav = HierarchyNavigator::new();
    let study = nav.study(&ds).unwrap();
    assert_eq!(study.study_instance_uid, "1.2.3.4.5");

    let series = nav.series(&ds).unwrap();
    assert_eq!(series.series_instance_uid, "1.2.3.4.5.6");

    let instance = nav.instance(&ds).unwrap();
    assert_eq!(instance.sop_instance_uid, "1.2.3.4.5.6.7");
}

#[test]
fn anonymization() {
    let mut ds = Dataset::new();
    ds.push(Element {
        tag: Tag::new(0x0010, 0x0010),
        vr: VR::Known(VRCode::PN),
        value: Value::String("Doe^John".into()),
    });
    ds.push(Element {
        tag: Tag::new(0x0010, 0x0020),
        vr: VR::Known(VRCode::LO),
        value: Value::String("12345".into()),
    });
    ds.push(Element {
        tag: Tag::new(0x0008, 0x0016),
        vr: VR::Known(VRCode::UI),
        value: Value::String("1.2.840.10008.5.1.4.1.1.2".into()),
    });

    let profile = AnonymizationProfile::basic();
    anonymize_dataset(&mut ds, &profile);

    // Patient-identifying tags removed.
    assert!(ds.get(Tag::new(0x0010, 0x0010)).is_none());
    assert!(ds.get(Tag::new(0x0010, 0x0020)).is_none());
    // Non-identifying tags kept.
    assert!(ds.get(Tag::new(0x0008, 0x0016)).is_some());
}

#[test]
fn empty_input_does_not_panic() {
    let _ = DicomReader::new().parse(&[]);
    let _ = DicomReader::new().parse(&[0u8; 10]);
    let _ = parse_dicom(&[], false);
}

#[test]
fn truncated_dicom_handled_gracefully() {
    // Preamble but no magic.
    let mut dcm = vec![0u8; 128];
    dcm.extend_from_slice(b"NOTDICM");
    let _ = DicomReader::new().parse(&dcm);

    // Preamble + magic but truncated dataset.
    let mut dcm2 = vec![0u8; 128];
    dcm2.extend_from_slice(b"DICM");
    dcm2.extend_from_slice(&0x0008u16.to_le_bytes());
    // Truncated — just tag group, no element.
    let _ = DicomReader::new().parse(&dcm2);
}
