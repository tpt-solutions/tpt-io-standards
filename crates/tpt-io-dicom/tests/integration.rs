use tpt_io_core::Reader;
use tpt_io_dicom::parser::DicomReader;

#[test]
fn round_trip_simple_element() {
    let mut dcm = Vec::new();
    dcm.extend_from_slice(&[0u8; 128]);
    dcm.extend_from_slice(b"DICM");

    dcm.extend_from_slice(&0x0002u16.to_le_bytes());
    dcm.extend_from_slice(&0x0010u16.to_le_bytes());
    dcm.extend_from_slice(b"UI");
    let ts = b"1.2.840.10008.1.2.1\0";
    dcm.extend_from_slice(&(ts.len() as u16).to_le_bytes());
    dcm.extend_from_slice(ts);

    dcm.extend_from_slice(&0x0008u16.to_le_bytes());
    dcm.extend_from_slice(&0x0016u16.to_le_bytes());
    dcm.extend_from_slice(b"UI");
    let sop = b"1.2.840.10008.5.1.4.1.1.2\0";
    dcm.extend_from_slice(&(sop.len() as u16).to_le_bytes());
    dcm.extend_from_slice(sop);

    let reader = DicomReader::new();
    let file = reader.parse(&dcm).unwrap();
    assert_eq!(
        file.dataset.string(tpt_io_dicom::Tag::new(0x0008, 0x0016)),
        Some("1.2.840.10008.5.1.4.1.1.2")
    );
}
