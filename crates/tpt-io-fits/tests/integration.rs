//! Integration tests for the FITS reader and writer, built from synthetic HDUs.

use tpt_io_fits::{card::Value, parse, write, Card, Fits, FitsError, Hdu};

fn card_image(s: &str) -> Vec<u8> {
    let mut v = s.as_bytes().to_vec();
    v.resize(80, b' ');
    v
}

/// A small primary HDU with 16-bit big-endian pixels, a scale and an offset.
fn scaled_primary() -> Vec<u8> {
    let mut header = Vec::new();
    for line in [
        "SIMPLE  =                    T / conforms to FITS standard",
        "BITPIX  =                   16",
        "NAXIS   =                    2",
        "NAXIS1  =                    3",
        "NAXIS2  =                    1",
        "BZERO   =                32768 / unsigned offset",
        "BSCALE  =                  2.0",
        "OBJECT  = 'M31     '           / target name",
        "COMMENT  synthetic test image",
    ] {
        header.extend_from_slice(&card_image(line));
    }
    header.extend_from_slice(&card_image("END"));
    header.resize(2880, b' ');

    let mut data = Vec::new();
    for raw in [0i16, 1, -1] {
        data.extend_from_slice(&raw.to_be_bytes());
    }
    data.resize(2880, 0);

    let mut file = header;
    file.extend_from_slice(&data);
    file
}

#[test]
fn parses_cards_and_decodes_scaled_pixels() {
    let f = parse(&scaled_primary()).unwrap();
    assert_eq!(f.hdus.len(), 1);
    let h = &f.hdus[0];
    assert_eq!(h.integer("BITPIX"), Some(16));
    assert_eq!(h.axes(), vec![3, 1]);
    assert_eq!(h.number("BSCALE"), Some(2.0));
    assert_eq!(h.card("OBJECT").unwrap().value, Some(Value::Text("M31".into())));
    assert_eq!(h.card("OBJECT").unwrap().comment.as_deref(), Some("target name"));
    assert_eq!(h.card("SIMPLE").unwrap().value, Some(Value::Logical(true)));
    // Raw signed values 0, 1, -1 with BZERO 32768 and BSCALE 2: bzero + bscale * raw.
    assert_eq!(h.pixels().unwrap(), vec![32768.0, 32770.0, 32766.0]);
}

#[test]
fn image_hdu_round_trips_through_write_and_parse() {
    let mut hdu = Hdu::image(-32, &[2, 2]);
    hdu.cards.push(Card::new("EXPTIME", Value::Real(12.5)));
    hdu.cards.push(Card::new("TELESCOP", Value::Text("Test 'Scope'".into())));
    hdu.cards.push(Card::commentary("HISTORY", "made by a test"));
    hdu.data = [1.5f32, -2.0, 0.25, 1e10].iter().flat_map(|v| v.to_be_bytes()).collect();
    let file = Fits { hdus: vec![hdu] };
    let bytes = write(&file);
    assert_eq!(bytes.len() % 2880, 0);
    let back = parse(&bytes).unwrap();
    assert_eq!(back, file);
    assert_eq!(back.hdus[0].pixels().unwrap(), vec![1.5, -2.0, 0.25, 1e10]);
}

#[test]
fn extensions_follow_the_primary() {
    let mut primary = Hdu::image(8, &[3]);
    primary.data = vec![7, 8, 9];
    let mut ext = Hdu::image(64, &[1]);
    ext.cards.push(Card::new("EXTNAME", Value::Text("SCI".into())));
    ext.data = (-5i64).to_be_bytes().to_vec();
    let bytes = write(&Fits { hdus: vec![primary.clone(), ext.clone()] });
    let back = parse(&bytes).unwrap();
    assert_eq!(back.hdus.len(), 2);
    assert_eq!(back.hdus[0].pixels().unwrap(), vec![7.0, 8.0, 9.0]);
    assert_eq!(back.hdus[1].card("EXTNAME").unwrap().value, Some(Value::Text("SCI".into())));
    assert_eq!(back.hdus[1].pixels().unwrap(), vec![-5.0]);
}

#[test]
fn truncated_and_unsupported_inputs_are_errors() {
    let full = scaled_primary();
    assert!(parse(&full[..100]).is_err());
    assert!(parse(&full[..2880]).is_err());

    let mut odd = Hdu::image(12, &[1]);
    odd.data = vec![0; 2];
    let f = Fits { hdus: vec![odd] };
    assert!(matches!(
        parse(&write(&f)).unwrap().hdus[0].pixels(),
        Err(FitsError::UnsupportedBitpix(12))
    ));
}

#[test]
fn card_parsing_handles_strings_reals_and_commentary() {
    let c = Card::parse(&card_image("EXPTIME =               1.5D3 / seconds")).unwrap();
    assert_eq!(c.value, Some(Value::Real(1500.0)));
    assert_eq!(c.comment.as_deref(), Some("seconds"));

    let s = Card::parse(&card_image("NAME    = 'it''s ok'")).unwrap();
    assert_eq!(s.value, Some(Value::Text("it's ok".into())));

    let h = Card::parse(&card_image("HISTORY reduced by hand")).unwrap();
    assert_eq!(h.keyword, "HISTORY");
    assert!(h.value.is_none());
    assert!(Card::parse(&card_image("BAD     = 'unterminated")).is_err());
}

#[test]
fn arbitrary_bytes_never_panic() {
    let mut x: u64 = 23;
    for _ in 0..300 {
        let len = (x % 6000) as usize;
        let bytes: Vec<u8> = (0..len)
            .map(|_| {
                x ^= x << 13;
                x ^= x >> 7;
                x ^= x << 17;
                b"SIMPLE  =TF/ 0123456789'END"[(x % 27) as usize]
            })
            .collect();
        let _ = parse(&bytes);
    }
}
