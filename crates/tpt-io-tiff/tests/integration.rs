//! Integration tests for the TIFF reader and writer.

use tpt_io_tiff::{parse, write, Image, TiffError};

fn grey(width: u32, height: u32, bits: u16, pixels: Vec<u8>) -> Image {
    Image {
        width,
        height,
        bits_per_sample: bits,
        samples_per_pixel: 1,
        sample_format: 1,
        description: None,
        pixels,
    }
}

#[test]
fn write_then_parse_keeps_every_ifd() {
    let a =
        Image { description: Some("first image".into()), ..grey(3, 2, 8, vec![1, 2, 3, 4, 5, 6]) };
    let b = Image { sample_format: 3, ..grey(2, 1, 32, vec![0, 0, 128, 63, 0, 0, 0, 64]) };
    let bytes = write(&[a.clone(), b.clone()]).unwrap();
    let back = parse(&bytes).unwrap();
    assert_eq!(back.len(), 2);
    assert_eq!(back[0], a);
    assert_eq!(back[1], b);
}

#[test]
fn sixteen_bit_and_rgb_images_round_trip() {
    let rgb = Image {
        width: 2,
        height: 1,
        bits_per_sample: 16,
        samples_per_pixel: 3,
        sample_format: 1,
        description: None,
        pixels: (0u16..6).flat_map(|v| (v * 1000).to_le_bytes()).collect(),
    };
    let back = parse(&write(std::slice::from_ref(&rgb)).unwrap()).unwrap();
    assert_eq!(back[0], rgb);
}

#[test]
fn big_endian_files_are_read() {
    // A 2x1 eight-bit image written in big-endian byte order by hand.
    let mut f = Vec::new();
    f.extend_from_slice(b"MM");
    f.extend_from_slice(&42u16.to_be_bytes());
    f.extend_from_slice(&8u32.to_be_bytes());
    let pixel_at = 8u32 + 2 + 6 * 12 + 4;
    f.extend_from_slice(&6u16.to_be_bytes());
    // SHORT values are left-justified in the 4-byte value field.
    let entry = |tag: u16, kind: u16, count: u32, value: u32| {
        let value = if kind == 3 { value << 16 } else { value };
        let mut e = Vec::new();
        e.extend_from_slice(&tag.to_be_bytes());
        e.extend_from_slice(&kind.to_be_bytes());
        e.extend_from_slice(&count.to_be_bytes());
        e.extend_from_slice(&value.to_be_bytes());
        e
    };
    f.extend(entry(256, 4, 1, 2));
    f.extend(entry(257, 4, 1, 1));
    f.extend(entry(258, 3, 1, 8));
    f.extend(entry(273, 4, 1, pixel_at));
    f.extend(entry(279, 4, 1, 2));
    f.extend(entry(277, 3, 1, 1));
    f.extend_from_slice(&0u32.to_be_bytes());
    f.extend_from_slice(&[10, 20]);
    let images = parse(&f).unwrap();
    assert_eq!(images[0].width, 2);
    assert_eq!(images[0].pixels, vec![10, 20]);
}

#[test]
fn lzw_strips_decode() {
    // TIFF LZW of the bytes 0x41 0x41 0x41 0x41 (the table learns "AA" and "AAA").
    let codes = [256u16, 65, 258, 257];
    let mut bits = Vec::new();
    for c in codes {
        for k in (0..9).rev() {
            bits.push(((c >> k) & 1) as u8);
        }
    }
    while bits.len() % 8 != 0 {
        bits.push(0);
    }
    let packed: Vec<u8> = bits.chunks(8).map(|c| c.iter().fold(0u8, |a, b| (a << 1) | b)).collect();
    let decoded = tpt_io_tiff::decode::lzw(&packed).unwrap();
    // Code 65 emits "A"; code 258 is the entry just added for "AA", so it emits "AA" too.
    assert_eq!(decoded, vec![65, 65, 65]);
}

#[test]
fn unsupported_and_malformed_files_are_errors() {
    assert!(matches!(parse(b"II+\0\x08\0\0\0"), Err(TiffError::Unsupported(_))));
    assert!(parse(b"XX*\0\x08\0\0\0").is_err());
    assert!(parse(b"II*\0").is_err());
    let mut bytes = write(&[grey(2, 2, 8, vec![1, 2, 3, 4])]).unwrap();
    bytes.truncate(bytes.len() - 6);
    assert!(parse(&bytes).is_err());
    assert!(write(&[grey(2, 2, 8, vec![1])]).is_err());
    assert!(write(&[grey(2, 2, 12, vec![0; 8])]).is_err());
}

#[test]
fn arbitrary_bytes_never_panic() {
    let mut x: u64 = 163;
    for _ in 0..300 {
        let len = (x % 300) as usize;
        let mut bytes: Vec<u8> = (0..len)
            .map(|_| {
                x ^= x << 13;
                x ^= x >> 7;
                x ^= x << 17;
                x as u8
            })
            .collect();
        if bytes.len() >= 4 {
            bytes[..4].copy_from_slice(b"II*\0");
        }
        let _ = parse(&bytes);
    }
}
