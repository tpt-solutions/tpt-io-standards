//! LAS header, point records, and writing.

use alloc::vec::Vec;

use tpt_io_core::ParseError;

use crate::error::LasError;

/// One point. Position is in the file's coordinate units, after scale and offset.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Point {
    /// Position `[x, y, z]`.
    pub position: [f64; 3],
    /// Intensity (16-bit).
    pub intensity: u16,
    /// Return number (1 to 7) from the flag byte.
    pub return_number: u8,
    /// Number of returns for this pulse.
    pub number_of_returns: u8,
    /// Classification code (bits 0–4 of byte 15).
    pub classification: u8,
    /// Scan angle rank, as stored.
    pub scan_angle: i8,
    /// User data byte.
    pub user_data: u8,
    /// Point source id.
    pub point_source_id: u16,
    /// GPS time, for formats 1 and 3.
    pub gps_time: Option<f64>,
    /// Colour `[red, green, blue]`, for formats 2 and 3.
    pub rgb: Option<[u16; 3]>,
}

/// A point cloud: its header values and points.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Cloud {
    /// LAS major version.
    pub version_major: u8,
    /// LAS minor version.
    pub version_minor: u8,
    /// Point data record format (0 to 3).
    pub point_format: u8,
    /// Scale factors for x, y and z.
    pub scale: [f64; 3],
    /// Offsets for x, y and z.
    pub offset: [f64; 3],
    /// The points in file order.
    pub points: Vec<Point>,
}

fn u16_at(b: &[u8], at: usize) -> Result<u16, ParseError> {
    b.get(at..at + 2)
        .map(|s| u16::from_le_bytes([s[0], s[1]]))
        .ok_or_else(|| ParseError::eof(at))
}

fn u32_at(b: &[u8], at: usize) -> Result<u32, ParseError> {
    b.get(at..at + 4)
        .map(|s| u32::from_le_bytes([s[0], s[1], s[2], s[3]]))
        .ok_or_else(|| ParseError::eof(at))
}

fn u64_at(b: &[u8], at: usize) -> Result<u64, ParseError> {
    let s = b.get(at..at + 8).ok_or_else(|| ParseError::eof(at))?;
    let mut a = [0u8; 8];
    a.copy_from_slice(s);
    Ok(u64::from_le_bytes(a))
}

fn f64_at(b: &[u8], at: usize) -> Result<f64, ParseError> {
    Ok(f64::from_bits(u64_at(b, at)?))
}

/// Parse a LAS file (uncompressed).
pub fn parse(input: &[u8]) -> Result<Cloud, LasError> {
    if input.len() < 227 || &input[..4] != b"LASF" {
        return Err(ParseError::custom(0, "missing LASF signature").into());
    }
    let version_major = input[24];
    let version_minor = input[25];
    let header_size = u16_at(input, 94)? as usize;
    let data_offset = u32_at(input, 96)? as usize;
    let point_format = input[104];
    let record_len = u16_at(input, 105)? as usize;
    let legacy_count = u32_at(input, 107)? as u64;
    let count = if version_minor >= 4 { u64_at(input, 247)? } else { legacy_count } as usize;
    if point_format > 3 {
        return Err(LasError::Unsupported(alloc::format!("point format {point_format}")));
    }
    if header_size < 227 || data_offset < header_size {
        return Err(ParseError::custom(94, "header or data offset is out of range").into());
    }
    let scale = [f64_at(input, 131)?, f64_at(input, 139)?, f64_at(input, 147)?];
    let offset = [f64_at(input, 155)?, f64_at(input, 163)?, f64_at(input, 171)?];
    let min_len = match point_format {
        0 => 20,
        1 => 28,
        2 => 26,
        _ => 34,
    };
    if record_len < min_len {
        return Err(ParseError::custom(105, "record length is shorter than the format needs").into());
    }
    let mut cloud = Cloud {
        version_major,
        version_minor,
        point_format,
        scale,
        offset,
        points: Vec::with_capacity(count),
    };
    for i in 0..count {
        let at = data_offset + i * record_len;
        let r = input
            .get(at..at + record_len)
            .ok_or_else(|| ParseError::eof(at))?;
        let x = i32::from_le_bytes([r[0], r[1], r[2], r[3]]) as f64 * scale[0] + offset[0];
        let y = i32::from_le_bytes([r[4], r[5], r[6], r[7]]) as f64 * scale[1] + offset[1];
        let z = i32::from_le_bytes([r[8], r[9], r[10], r[11]]) as f64 * scale[2] + offset[2];
        let flags = r[14];
        let mut p = Point {
            position: [x, y, z],
            intensity: u16::from_le_bytes([r[12], r[13]]),
            return_number: flags & 0x07,
            number_of_returns: (flags >> 3) & 0x07,
            classification: r[15] & 0x1F,
            scan_angle: r[16] as i8,
            user_data: r[17],
            point_source_id: u16::from_le_bytes([r[18], r[19]]),
            gps_time: None,
            rgb: None,
        };
        if point_format == 1 || point_format == 3 {
            p.gps_time = Some(f64::from_bits(u64::from_le_bytes([r[20], r[21], r[22], r[23], r[24], r[25], r[26], r[27]])));
        }
        if point_format == 2 {
            p.rgb = Some([u16::from_le_bytes([r[20], r[21]]), u16::from_le_bytes([r[22], r[23]]), u16::from_le_bytes([r[24], r[25]])]);
        }
        if point_format == 3 {
            p.rgb = Some([u16::from_le_bytes([r[28], r[29]]), u16::from_le_bytes([r[30], r[31]]), u16::from_le_bytes([r[32], r[33]])]);
        }
        cloud.points.push(p);
    }
    Ok(cloud)
}

/// Serialise a cloud as LAS 1.2 with point format 0 or 1. Each coordinate is stored as
/// `round((value - offset) / scale)`; a value that does not fit 32 bits is rejected.
pub fn write(c: &Cloud) -> Result<Vec<u8>, LasError> {
    if c.point_format > 1 {
        return Err(LasError::Unsupported("writing point formats above 1".into()));
    }
    let record_len: u16 = if c.point_format == 1 { 28 } else { 20 };
    let header_size: u16 = 227;
    let data_offset = header_size as u32;
    let mut out = alloc::vec![0u8; header_size as usize];
    out[..4].copy_from_slice(b"LASF");
    out[24] = 1;
    out[25] = 2;
    out[94..96].copy_from_slice(&header_size.to_le_bytes());
    out[96..100].copy_from_slice(&data_offset.to_le_bytes());
    out[104] = c.point_format;
    out[105..107].copy_from_slice(&record_len.to_le_bytes());
    let count = c.points.len() as u32;
    out[107..111].copy_from_slice(&count.to_le_bytes());
    for i in 0..3 {
        out[131 + 8 * i..139 + 8 * i].copy_from_slice(&c.scale[i].to_le_bytes());
        out[155 + 8 * i..163 + 8 * i].copy_from_slice(&c.offset[i].to_le_bytes());
    }
    for p in &c.points {
        let mut r = alloc::vec![0u8; record_len as usize];
        for axis in 0..3 {
            let raw = ((p.position[axis] - c.offset[axis]) / c.scale[axis]).round();
            if !(raw >= i32::MIN as f64 && raw <= i32::MAX as f64) {
                return Err(LasError::Invalid("coordinate does not fit the scale and offset".into()));
            }
            r[4 * axis..4 * axis + 4].copy_from_slice(&(raw as i32).to_le_bytes());
        }
        r[12..14].copy_from_slice(&p.intensity.to_le_bytes());
        r[14] = (p.return_number & 0x07) | ((p.number_of_returns & 0x07) << 3);
        r[15] = p.classification & 0x1F;
        r[16] = p.scan_angle as u8;
        r[17] = p.user_data;
        r[18..20].copy_from_slice(&p.point_source_id.to_le_bytes());
        if c.point_format == 1 {
            r[20..28].copy_from_slice(&p.gps_time.unwrap_or(0.0).to_bits().to_le_bytes());
        }
        out.extend_from_slice(&r);
    }
    Ok(out)
}
