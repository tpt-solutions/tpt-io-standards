//! Integration tests for the ASTERIX framing and CAT048 decoding.

use tpt_io_asterix::{parse, AsterixError, Record};

/// Build a CAT048 block from a list of records, each given as FSPEC octets plus item bytes.
fn block(category: u8, records: &[(&[u8], &[u8])]) -> Vec<u8> {
    let mut body = Vec::new();
    for (fspec, items) in records {
        body.extend_from_slice(fspec);
        body.extend_from_slice(items);
    }
    let len = (body.len() + 3) as u16;
    let mut out = vec![category];
    out.extend_from_slice(&len.to_be_bytes());
    out.extend(body);
    out
}

#[test]
fn decodes_a_cat048_record_with_four_items() {
    // FRN 1 → bit 0x80; FRN 2 → 0x40; FRN 4 → 0x10; FRN 5 → 0x08; FX (0x01) clear.
    let fspec = [0x80 | 0x40 | 0x10 | 0x08];
    let items = [
        0x01, 0x02, // I048/010: SAC 1, SIC 2
        0x00, 0x00, 0x80, // I048/140: 128 → 1.0 s
        0x01, 0x00, 0x20, 0x00, // I048/040: rho 256/256 = 1 NM, theta 0x2000 → 45 degrees
        0x12, 0x34, // I048/070: masked to 0x234
    ];
    let bytes = block(48, &[(&fspec, &items)]);
    let blocks = parse(&bytes).unwrap();
    assert_eq!(blocks[0].category, 48);
    let Record::Cat048(r) = &blocks[0].records[0];
    assert_eq!(r.data_source, Some((1, 2)));
    assert_eq!(r.time_of_day, Some(128));
    assert_eq!(blocks[0].records[0].time_of_day_s(), Some(1.0));
    assert_eq!(r.polar_position, Some((1.0, 45.0)));
    assert_eq!(r.mode3a, Some(0x234));
}

#[test]
fn fx_extends_the_fspec_across_octets() {
    // Octet 1 sets FRN 1 and FX; octet 2 sets FRN 8 (its first bit, index 7 → 7*1+1... see frn map)
    // FRN 8 is the first bit of the second octet: 0x80 → frn = 1*7 + 0 + 1 = 8.
    let fspec = [0x81u8, 0x80];
    let items = [0x00, 0x01, 0xAB, 0xCD, 0xEF];
    let bytes = block(48, &[(&fspec, &items)]);
    let blocks = parse(&bytes).unwrap();
    let Record::Cat048(r) = &blocks[0].records[0];
    assert_eq!(r.data_source, Some((0, 1)));
    assert_eq!(r.aircraft_address, Some(0xABCDEF));
}

#[test]
fn several_records_in_one_block() {
    let fspec = [0x40u8]; // FRN 2 only
    let a = [0x00, 0x00, 0x01];
    let b = [0x00, 0x00, 0x02];
    let bytes = block(48, &[(&fspec, &a), (&fspec, &b)]);
    let blocks = parse(&bytes).unwrap();
    assert_eq!(blocks[0].records.len(), 2);
    assert_eq!(blocks[0].records[1].time_of_day_s(), Some(2.0 / 128.0));
}

#[test]
fn unsupported_items_are_reported_not_guessed() {
    let fspec = [0x20u8]; // FRN 3 (target report descriptor, variable length) is not supported
    let bytes = block(48, &[(&fspec, &[0x00])]);
    assert_eq!(
        parse(&bytes),
        Err(AsterixError::UnsupportedItem { category: 48, frn: 3 })
    );
    let other = block(21, &[(&[0x80u8], &[0, 0])]);
    assert!(matches!(parse(&other), Err(AsterixError::UnsupportedItem { category: 21, .. })));
}

#[test]
fn bad_lengths_and_truncation_are_errors() {
    assert!(parse(&[48, 0x00]).is_err());
    assert!(parse(&[48, 0x00, 0x02]).is_err());
    assert!(parse(&[48, 0x00, 0x20, 0x80]).is_err());
    let fspec = [0x40u8];
    let mut bytes = block(48, &[(&fspec, &[0x00, 0x01, 0x02])]);
    bytes.truncate(bytes.len() - 1);
    assert!(parse(&bytes).is_err());
}

#[test]
fn arbitrary_bytes_never_panic() {
    let mut x: u64 = 31;
    for _ in 0..400 {
        let len = (x % 120) as usize;
        let bytes: Vec<u8> = (0..len)
            .map(|_| {
                x ^= x << 13;
                x ^= x >> 7;
                x ^= x << 17;
                x as u8
            })
            .collect();
        let _ = parse(&bytes);
    }
}
