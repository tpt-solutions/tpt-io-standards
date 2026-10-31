//! Integration tests for the mzML reader.

use tpt_io_mzml::{parse, MzmlError};

/// Base64 of little-endian f64 values 100.5 and 200.25 (m/z), and f32 values 10 and 20 (intensity).
fn b64(bytes: &[u8]) -> String {
    const A: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    for chunk in bytes.chunks(3) {
        let b = [chunk[0], *chunk.get(1).unwrap_or(&0), *chunk.get(2).unwrap_or(&0)];
        let n = ((b[0] as u32) << 16) | ((b[1] as u32) << 8) | b[2] as u32;
        out.push(A[(n >> 18) as usize & 63] as char);
        out.push(A[(n >> 12) as usize & 63] as char);
        out.push(if chunk.len() > 1 { A[(n >> 6) as usize & 63] as char } else { '=' });
        out.push(if chunk.len() > 2 { A[n as usize & 63] as char } else { '=' });
    }
    out
}

fn document() -> Vec<u8> {
    let mz: Vec<u8> = [100.5f64, 200.25].iter().flat_map(|v| v.to_le_bytes()).collect();
    let inten: Vec<u8> = [10.0f32, 20.0].iter().flat_map(|v| v.to_le_bytes()).collect();
    format!(
        r#"<?xml version="1.0"?>
<mzML xmlns="http://psi.hupo.org/ms/mzml">
  <run id="run1">
    <spectrumList count="2">
      <spectrum index="0" id="scan=1" defaultArrayLength="2">
        <cvParam accession="MS:1000511" name="ms level" value="1"/>
        <scanList count="1"><scan>
          <cvParam accession="MS:1000016" name="scan start time" value="0.5" unitName="minute"/>
        </scan></scanList>
        <binaryDataArrayList count="2">
          <binaryDataArray encodedLength="32">
            <cvParam accession="MS:1000523" name="64-bit float"/>
            <cvParam accession="MS:1000576" name="no compression"/>
            <cvParam accession="MS:1000514" name="m/z array"/>
            <binary>{}</binary>
          </binaryDataArray>
          <binaryDataArray encodedLength="16">
            <cvParam accession="MS:1000521" name="32-bit float"/>
            <cvParam accession="MS:1000576" name="no compression"/>
            <cvParam accession="MS:1000515" name="intensity array"/>
            <binary>{}</binary>
          </binaryDataArray>
        </binaryDataArrayList>
      </spectrum>
      <spectrum index="1" id="scan=2" defaultArrayLength="0">
        <cvParam accession="MS:1000511" name="ms level" value="2"/>
      </spectrum>
    </spectrumList>
  </run>
</mzML>"#,
        b64(&mz),
        b64(&inten)
    )
    .into_bytes()
}

#[test]
fn decodes_spectra_arrays_and_metadata() {
    let run = parse(&document()).unwrap();
    assert_eq!(run.id.as_deref(), Some("run1"));
    assert_eq!(run.spectra.len(), 2);
    let s = &run.spectra[0];
    assert_eq!(s.id, "scan=1");
    assert_eq!(s.ms_level, Some(1));
    assert_eq!(s.retention_time, Some(30.0));
    assert_eq!(s.mz, vec![100.5, 200.25]);
    assert_eq!(s.intensity, vec![10.0, 20.0]);
    assert_eq!(run.spectra[1].ms_level, Some(2));
    assert!(run.spectra[1].mz.is_empty());
}

#[test]
fn zlib_arrays_are_reported_not_decoded() {
    let doc = br#"<mzML><run><spectrumList><spectrum id="s">
      <binaryDataArrayList><binaryDataArray>
        <cvParam accession="MS:1000574" name="zlib compression"/>
        <cvParam accession="MS:1000523" name="64-bit float"/>
        <cvParam accession="MS:1000514" name="m/z array"/>
        <binary>AAAA</binary>
      </binaryDataArray></binaryDataArrayList></spectrum></spectrumList></run></mzML>"#;
    assert!(matches!(parse(doc), Err(MzmlError::Unsupported(_))));
}

#[test]
fn bad_documents_are_errors() {
    assert!(parse(b"<other/>").is_err());
    assert!(matches!(parse(b"<mzML></mzML>"), Err(MzmlError::Invalid(_))));
    assert!(parse(b"<mzML><run><spectrumList><spectrum/></spectrumList></run></mzML>").is_err());
    assert!(parse(b"<mzML><run><spectrumList>").is_err());
}

#[test]
fn base64_decoder_round_trips_known_text() {
    assert_eq!(tpt_io_mzml::base64::decode("AQIDBA==").unwrap(), vec![1, 2, 3, 4]);
}

#[test]
fn arbitrary_bytes_never_panic() {
    let mut x: u64 = 73;
    for _ in 0..200 {
        let len = (x % 240) as usize;
        let bytes: Vec<u8> = (0..len)
            .map(|_| {
                x ^= x << 13;
                x ^= x >> 7;
                x ^= x << 17;
                let alphabet = b"<mzML>spectrum binary=\"/ AB0+=\n";
                alphabet[(x % alphabet.len() as u64) as usize]
            })
            .collect();
        let _ = parse(&bytes);
    }
}
