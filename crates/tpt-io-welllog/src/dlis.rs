//! DLIS (Digital Log Interchange Standard, RP66 V1) parsing.
//!
//! DLIS stores well-log data in a *frame-based* organization: a Storage Unit
//! Label is followed by a sequence of Visible Records, each containing Logical
//! Records. Set objects (`FRAME`, `CHANNEL`, `WELL`, `WELLBORE`, `TOOL`, …)
//! describe the structure, and `FRAME` objects reference the `CHANNEL`s that
//! make up each row of data. Actual channel values are carried in per-frame
//! data records.
//!
//! This implementation parses the full container structure (SUL → Visible
//! Records → Logical Record Segments), decodes Set objects via the standard
//! attribute representation (LABEL + REPRESENTATION-CODE + VALUE, terminated by
//! a zero rep-code), and reads frame data for the common single-precision /
//! integer explicit and implicit formats (`FSINGL`, `ISINGL`, `FDOUBL`,
//! `IDOUBL`, `ISLONG`, `ISHORT`). Multi-frame correlation is handled by the
//! caller via the typed [`Dlis`] model.

use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

use crate::error::WellLogError;

/// RP66 V1 Storage Unit Label (first 80 bytes of the file).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct StorageUnitLabel {
    /// Storage Unit Sequence Number (first 4 bytes, binary).
    pub sequence_number: u32,
    /// Storage Unit Structure (should be `"RECORD"`).
    pub structure: String,
    /// Maximum Record Length (bytes).
    pub max_record_length: u32,
    /// Storage Set Identifier (should be `'S'`).
    pub set_identifier: u8,
    /// Version (should be `'V'`).
    pub version: u8,
    /// Storage Unit Label flags.
    pub flags: u16,
}

/// A value decoded from a DLIS representation code.
#[derive(Debug, Clone, PartialEq)]
pub enum RepValue {
    /// String identifier (`IDENT` / `ASCII`).
    Ident(String),
    /// Signed integer (any width up to 64 bits).
    Int(i64),
    /// 32-bit IEEE float.
    Float(f32),
    /// 64-bit IEEE double.
    Double(f64),
    /// Unsigned integer (any width up to 64 bits).
    Uint(u64),
    /// An object name: ident + origin + copy.
    Obname(Obname),
    /// Raw, uninterpreted bytes (unknown representation code).
    Raw(Vec<u8>),
}

/// An object name: `IDENT` + origin + copy number.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Obname {
    /// Identifying string.
    pub ident: String,
    /// Origin (typically the file-unique object number space).
    pub origin: i16,
    /// Copy number.
    pub copy: u16,
}

/// A parsed Set object (e.g. a `FRAME` or `CHANNEL`).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SetObject {
    /// Set type, e.g. `"FRAME"`, `"CHANNEL"`, `"WELL"`.
    pub set_type: String,
    /// Object name string (ident part of the OBNAME).
    pub name: String,
    /// Full object name.
    pub obname: Obname,
    /// Attribute (label → value) list, in file order.
    pub attributes: Vec<(String, RepValue)>,
}

impl SetObject {
    /// Look up the first attribute value by label (case-insensitive).
    pub fn attr(&self, label: &str) -> Option<&RepValue> {
        let l = label.to_ascii_uppercase();
        self.attributes.iter().find(|(k, _)| k.to_ascii_uppercase() == l).map(|(_, v)| v)
    }
}

/// A single row of frame data (one value per channel).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct FrameSample {
    /// Frame number within the run.
    pub frame_number: u32,
    /// One value per channel, in the frame's channel order.
    pub values: Vec<f64>,
}

/// A DLIS `FRAME` object: the column layout plus decoded samples.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Frame {
    /// Frame name/identifier.
    pub name: String,
    /// Channels referenced by this frame (in column order).
    pub channels: Vec<Obname>,
    /// Index curve OBNAME, if declared.
    pub index: Option<Obname>,
    /// Decoded samples.
    pub samples: Vec<FrameSample>,
}

/// A DLIS `CHANNEL` object.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Channel {
    /// Channel name/identifier (mnemonic-equivalent).
    pub name: String,
    /// Long name (human-readable).
    pub long_name: Option<String>,
    /// Physical units.
    pub units: Option<String>,
    /// Representation code (RP66 rep-code of stored values).
    pub rep_code: Option<u16>,
    /// Dimension (number of values per sample for array channels).
    pub dimension: Option<u32>,
    /// Number of samples per frame.
    pub samples: Option<u32>,
}

/// A fully parsed DLIS file.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Dlis {
    /// Storage Unit Label.
    pub label: StorageUnitLabel,
    /// Number of logical records seen.
    pub logical_record_count: usize,
    /// File header set object, if present.
    pub file_header: Option<SetObject>,
    /// Well set objects.
    pub wells: Vec<SetObject>,
    /// Wellbore set objects.
    pub wellbores: Vec<SetObject>,
    /// Tool set objects.
    pub tools: Vec<SetObject>,
    /// Channel set objects (also surfaced as [`Dlis::channels`]).
    pub channel_objects: Vec<SetObject>,
    /// Frame set objects (with decoded samples).
    pub frames: Vec<Frame>,
    /// Typed channel metadata.
    pub channels: Vec<Channel>,
}

impl Dlis {
    /// Total number of decoded frame samples across all frames.
    pub fn sample_count(&self) -> usize {
        self.frames.iter().map(|f| f.samples.len()).sum()
    }
}

/// Logical record types that carry Set objects.
const SET_TYPES: &[u8] = &[0, 1, 2, 3, 9, 49, 50, 51, 52, 53];
/// Logical record types that carry frame data (explicit/implicit formats).
/// 1=FSINGL(F), 3=ISINGL(I), 5=FDOUBL(F), 7=IDOUBL(I), 11=ISLONG(I), 15=ISHORT(I).
const FRAME_DATA_TYPES: &[u8] = &[1, 3, 5, 7, 11, 15];

/// Parse a DLIS (RP66 V1) byte buffer into a [`Dlis`].
pub fn parse(input: &[u8]) -> Result<Dlis, WellLogError> {
    let mut dlis = Dlis::default();
    let mut pos = 80usize;

    // Storage Unit Label (80 bytes).
    if input.len() < 80 {
        return Err(WellLogError::Unsupported("DLIS file shorter than 80-byte Storage Unit Label"));
    }
    dlis.label = parse_sul(&input[..80])?;

    let mut current_frame: Option<usize> = None;

    while pos + 4 <= input.len() {
        // Visible Record Header: 2-byte length (excludes header) + 2-byte checksum.
        let rec_len = u16::from_be_bytes([input[pos], input[pos + 1]]) as usize;
        // input[pos+2], input[pos+3] = checksum (unused here)
        pos += 4;
        if rec_len == 0 {
            break;
        }
        let rec_end = core::cmp::min(pos + rec_len, input.len());
        let body = &input[pos..rec_end];
        pos = rec_end;

        // Logical Record Segments within the visible record.
        let mut seg = 0usize;
        while seg + 4 <= body.len() {
            let seg_len = u16::from_be_bytes([body[seg], body[seg + 1]]) as usize;
            let seg_type = body[seg + 2];
            // body[seg+3] = attributes (encryption/etc.), ignored.
            seg += 4;
            if seg_len == 0 {
                break;
            }
            let seg_end = core::cmp::min(seg + seg_len, body.len());
            let data = &body[seg..seg_end];
            seg = seg_end;
            dlis.logical_record_count += 1;

            if FRAME_DATA_TYPES.contains(&seg_type) {
                // Frame-data format codes (1,3,5,7,11,15) overlap the Set-type
                // codes for Originating-Organization (1) and Wellbore (3). We try
                // the frame-data interpretation first; if it fails we fall back to
                // the Set interpretation so genuine SET objects are still parsed.
                match parse_frame_data(&mut dlis, seg_type, data, current_frame) {
                    Ok(()) => {}
                    Err(_) if SET_TYPES.contains(&seg_type) => {
                        parse_set_record(&mut dlis, seg_type, data, &mut current_frame)?;
                    }
                    Err(e) => return Err(e),
                }
            } else if SET_TYPES.contains(&seg_type) {
                parse_set_record(&mut dlis, seg_type, data, &mut current_frame)?;
            }
            // Other record types (dictionaries, etc.) are skipped.
        }
    }

    Ok(dlis)
}

/// Parse the 80-byte Storage Unit Label.
fn parse_sul(b: &[u8]) -> Result<StorageUnitLabel, WellLogError> {
    let sequence_number = u32::from_be_bytes([b[0], b[1], b[2], b[3]]);
    // bytes 4..64 = 60-byte structure field (ASCII "RECORD").
    let structure = String::from_utf8_lossy(&b[4..64]).trim_end().to_string();
    let max_record_length = u32::from_be_bytes([b[64], b[65], b[66], b[67]]);
    let set_identifier = b[68];
    let version = b[69];
    let flags = u16::from_be_bytes([b[70], b[71]]);
    Ok(StorageUnitLabel {
        sequence_number,
        structure,
        max_record_length,
        set_identifier,
        version,
        flags,
    })
}

/// Parse a Set-object logical record: a SET-TYPE (IDENT), a COUNT (ULONG), then
/// `count` objects each = OBNAME + attribute list terminated by rep-code 0.
fn parse_set_record(
    dlis: &mut Dlis,
    set_type_code: u8,
    data: &[u8],
    current_frame: &mut Option<usize>,
) -> Result<(), WellLogError> {
    let mut c = Cursor::new(data);

    // SET-TYPE (IDENT) then COUNT (ULONG).
    let rc_type = c.read_u16()?;
    if rc_type != 1 {
        return Err(WellLogError::Unsupported("DLIS SET record missing IDENT set-type"));
    }
    let set_type = c.read_value(1)?.into_string();

    let rc_count = c.read_u16()?;
    if rc_count != 8 {
        return Err(WellLogError::Unsupported("DLIS SET record missing ULONG count"));
    }
    let count = match c.read_value(8)? {
        RepValue::Uint(n) => n as usize,
        _ => return Err(WellLogError::Unsupported("DLIS SET count not unsigned")),
    };

    for _ in 0..count {
        let rc_name = c.read_u16()?;
        if rc_name != 12 {
            return Err(WellLogError::Unsupported("DLIS SET object missing OBNAME"));
        }
        let obname = match c.read_value(12)? {
            RepValue::Obname(o) => o,
            _ => return Err(WellLogError::Unsupported("DLIS OBNAME decode failed")),
        };
        let obj = SetObject {
            set_type: set_type.clone(),
            name: obname.ident.clone(),
            obname: obname.clone(),
            attributes: parse_attributes(&mut c)?,
        };
        classify(dlis, obj, set_type_code, current_frame);
    }
    Ok(())
}

/// Parse an object's attribute list: triples of (LABEL IDENT, REP-CODE, VALUE)
/// until a zero rep-code terminator.
fn parse_attributes(c: &mut Cursor<'_>) -> Result<Vec<(String, RepValue)>, WellLogError> {
    let mut attrs = Vec::new();
    loop {
        let rc = c.read_u16()?;
        if rc == 0 {
            break; // terminator
        }
        // LABEL is an IDENT.
        if rc != 1 {
            return Err(WellLogError::Unsupported("DLIS attribute missing IDENT label"));
        }
        let label = c.read_value(1)?.into_string();
        let value_rc = c.read_u16()?;
        let value = c.read_value(value_rc)?;
        attrs.push((label, value));
    }
    Ok(attrs)
}

/// Route a parsed Set object into the correct [`Dlis`] collection.
fn classify(dlis: &mut Dlis, obj: SetObject, set_type_code: u8, current_frame: &mut Option<usize>) {
    match set_type_code {
        0 => dlis.file_header = Some(obj),
        2 => dlis.wells.push(obj),
        3 => dlis.wellbores.push(obj),
        9 => dlis.tools.push(obj),
        50 => {
            let ch = channel_from_object(&obj);
            dlis.channels.push(ch);
            dlis.channel_objects.push(obj);
        }
        49 => {
            let channels = match obj.attr("CHANNELS") {
                Some(RepValue::Obname(o)) => core::slice::from_ref(o).to_vec(),
                _ => Vec::new(),
            };
            let index = match obj.attr("INDEX") {
                Some(RepValue::Obname(o)) => Some(o.clone()),
                _ => None,
            };
            let frame = Frame { name: obj.name.clone(), channels, index, samples: Vec::new() };
            *current_frame = Some(dlis.frames.len());
            dlis.frames.push(frame);
        }
        _ => { /* other set types retained only via raw records */ }
    }
}

/// Build typed [`Channel`] metadata from a `CHANNEL` Set object.
fn channel_from_object(obj: &SetObject) -> Channel {
    let long_name =
        obj.attr("LONG-NAME").or_else(|| obj.attr("LONG_NAME")).map(|v| v.clone().into_string());
    let units = obj.attr("UNITS").map(|v| v.clone().into_string());
    let rep_code = obj.attr("REPRESENTATION-CODE").and_then(|v| match v {
        RepValue::Uint(n) => Some(*n as u16),
        _ => None,
    });
    let dimension = obj.attr("DIMENSION").and_then(|v| match v {
        RepValue::Uint(n) => Some(*n as u32),
        _ => None,
    });
    let samples = obj.attr("SAMPLES").and_then(|v| match v {
        RepValue::Uint(n) => Some(*n as u32),
        _ => None,
    });
    Channel { name: obj.name.clone(), long_name, units, rep_code, dimension, samples }
}

/// Parse a frame-data logical record (explicit = one frame, implicit = many).
fn parse_frame_data(
    dlis: &mut Dlis,
    rec_type: u8,
    data: &[u8],
    current_frame: Option<usize>,
) -> Result<(), WellLogError> {
    let implicit = matches!(rec_type, 3 | 7 | 11 | 15);
    let elem_size = match rec_type {
        1 | 3 => 4,   // f32 / i32
        5 | 7 => 8,   // f64 / i64
        11 | 15 => 2, // i16 (value slot)
        _ => 4,
    };

    let frame_idx = match current_frame {
        Some(i) => i,
        None => return Ok(()), // no frame to attach to
    };
    let n_channels = dlis.frames[frame_idx].channels.len().max(1);

    let mut c = Cursor::new(data);
    let read_elem = |c: &mut Cursor<'_>| -> Result<f64, WellLogError> {
        match rec_type {
            1 | 5 => {
                let b = c.take(elem_size)?;
                if elem_size == 4 {
                    Ok(f32::from_be_bytes(b.try_into().unwrap()) as f64)
                } else {
                    Ok(f64::from_be_bytes(b.try_into().unwrap()))
                }
            }
            3 | 7 => {
                let b = c.take(elem_size)?;
                if elem_size == 4 {
                    Ok(i32::from_be_bytes(b.try_into().unwrap()) as f64)
                } else {
                    Ok(i64::from_be_bytes(b.try_into().unwrap()) as f64)
                }
            }
            11 | 15 => {
                let b = c.take(2)?;
                Ok(i16::from_be_bytes(b.try_into().unwrap()) as f64)
            }
            _ => Err(WellLogError::Unsupported("unsupported DLIS frame format")),
        }
    };

    let total_frames = if implicit { (c.read_u16()? as usize).min(1 << 20) } else { 1 };

    for _ in 0..total_frames {
        let frame_number = c.read_u16()? as u32;
        let mut values = Vec::with_capacity(n_channels);
        for _ in 0..n_channels {
            values.push(read_elem(&mut c)?);
        }
        dlis.frames[frame_idx].samples.push(FrameSample { frame_number, values });
    }
    Ok(())
}

impl RepValue {
    fn into_string(self) -> String {
        match self {
            RepValue::Ident(s) => s,
            RepValue::Int(n) => n.to_string(),
            RepValue::Uint(n) => n.to_string(),
            RepValue::Float(f) => f.to_string(),
            RepValue::Double(d) => d.to_string(),
            other => format!("{other:?}"),
        }
    }
}

/// A simple bounds-checked byte cursor.
struct Cursor<'a> {
    buf: &'a [u8],
    pos: usize,
}

impl<'a> Cursor<'a> {
    fn new(buf: &'a [u8]) -> Self {
        Self { buf, pos: 0 }
    }

    fn read_u16(&mut self) -> Result<u16, WellLogError> {
        if self.pos + 2 > self.buf.len() {
            return Err(WellLogError::Unsupported("DLIS truncated u16"));
        }
        let v = u16::from_be_bytes([self.buf[self.pos], self.buf[self.pos + 1]]);
        self.pos += 2;
        Ok(v)
    }

    fn take(&mut self, n: usize) -> Result<&'a [u8], WellLogError> {
        if self.pos + n > self.buf.len() {
            return Err(WellLogError::Unsupported("DLIS truncated value"));
        }
        let s = &self.buf[self.pos..self.pos + n];
        self.pos += n;
        Ok(s)
    }

    /// Decode a value of the given RP66 representation code.
    fn read_value(&mut self, code: u16) -> Result<RepValue, WellLogError> {
        match code {
            1 | 2 => {
                // IDENT / ASCII: 2-byte length + bytes.
                let len = self.read_u16()? as usize;
                let b = self.take(len)?;
                Ok(RepValue::Ident(String::from_utf8_lossy(b).trim_end().to_string()))
            }
            3 => {
                let b = self.take(4)?;
                Ok(RepValue::Int(i32::from_be_bytes(b.try_into().unwrap()) as i64))
            }
            4 => {
                let b = self.take(4)?;
                Ok(RepValue::Float(f32::from_be_bytes(b.try_into().unwrap())))
            }
            5 => {
                let b = self.take(8)?;
                Ok(RepValue::Double(f64::from_be_bytes(b.try_into().unwrap())))
            }
            6 => {
                let b = self.take(2)?;
                Ok(RepValue::Int(i16::from_be_bytes(b.try_into().unwrap()) as i64))
            }
            7 => {
                let b = self.take(1)?;
                Ok(RepValue::Int(b[0] as i8 as i64))
            }
            8 => {
                let b = self.take(4)?;
                Ok(RepValue::Uint(u32::from_be_bytes(b.try_into().unwrap()) as u64))
            }
            9 => {
                let b = self.take(2)?;
                Ok(RepValue::Uint(u16::from_be_bytes(b.try_into().unwrap()) as u64))
            }
            11 => {
                // ORIGIN: 2-byte signed.
                let b = self.take(2)?;
                Ok(RepValue::Int(i16::from_be_bytes(b.try_into().unwrap()) as i64))
            }
            12 => {
                // OBNAME: IDENT + ORIGIN(2) + COPY(2).
                let ident = match self.read_value(1)? {
                    RepValue::Ident(s) => s,
                    _ => String::new(),
                };
                let origin = {
                    let b = self.take(2)?;
                    i16::from_be_bytes(b.try_into().unwrap())
                };
                let copy = {
                    let b = self.take(2)?;
                    u16::from_be_bytes(b.try_into().unwrap())
                };
                Ok(RepValue::Obname(Obname { ident, origin, copy }))
            }
            10 | 13 | 14 | 15 | 16 => {
                // UVAR / OBJREF / ATTREF / STATUS / TIME: read a fixed small chunk.
                let len = match code {
                    10 => 4,
                    13 | 14 => 8,
                    15 => 1,
                    16 => 8,
                    _ => 4,
                };
                let b = self.take(len)?;
                Ok(RepValue::Raw(b.to_vec()))
            }
            _ => {
                // Unknown: capture 4 bytes best-effort.
                let b = self.take(4).unwrap_or(&[]);
                Ok(RepValue::Raw(b.to_vec()))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build_minimal_dlis() -> Vec<u8> {
        let mut b: Vec<u8> = Vec::new();
        // --- Storage Unit Label (80 bytes) ---
        b.extend_from_slice(&1u32.to_be_bytes()); // sequence number
        let mut structure = [b' '; 60];
        structure[..6].copy_from_slice(b"RECORD");
        b.extend_from_slice(&structure);
        b.extend_from_slice(&80u32.to_be_bytes()); // max record length
        b.push(b'S'); // set identifier
        b.push(b'V'); // version
        b.extend_from_slice(&0u16.to_be_bytes()); // flags
        b.extend_from_slice(&[0u8; 8]); // storage set security

        // --- Frame set (type 49) logical record ---
        let mut lr = Vec::new();
        lr.extend_from_slice(&1u16.to_be_bytes()); // rep code: IDENT
        lr.extend_from_slice(&5u16.to_be_bytes()); // ident length
        lr.extend_from_slice(b"FRAME");
        lr.extend_from_slice(&8u16.to_be_bytes()); // rep code: ULONG
        lr.extend_from_slice(&1u32.to_be_bytes()); // count = 1
                                                   // object OBNAME (rep 12)
        lr.extend_from_slice(&12u16.to_be_bytes());
        lr.extend_from_slice(&3u16.to_be_bytes()); // ident length
        lr.extend_from_slice(b"FR1");
        lr.extend_from_slice(&0i16.to_be_bytes()); // origin
        lr.extend_from_slice(&1u16.to_be_bytes()); // copy
                                                   // attributes: LABEL("CHANNELS") = OBNAME DEPT, then terminator 0
        lr.extend_from_slice(&1u16.to_be_bytes()); // rep: IDENT (label)
        lr.extend_from_slice(&8u16.to_be_bytes()); // label length
        lr.extend_from_slice(b"CHANNELS");
        lr.extend_from_slice(&12u16.to_be_bytes()); // value rep: OBNAME
        lr.extend_from_slice(&4u16.to_be_bytes()); // ident length
        lr.extend_from_slice(b"DEPT");
        lr.extend_from_slice(&0i16.to_be_bytes());
        lr.extend_from_slice(&1u16.to_be_bytes());
        lr.extend_from_slice(&0u16.to_be_bytes()); // terminator

        // Wrap the logical record in a segment header (type 49 = FRAME set),
        // then in a visible record.
        let mut seg = Vec::new();
        seg.extend_from_slice(&(lr.len() as u16).to_be_bytes()); // segment length
        seg.push(49); // segment type
        seg.push(0); // attributes
        seg.extend_from_slice(&lr);

        let mut vr = Vec::new();
        vr.extend_from_slice(&(seg.len() as u16).to_be_bytes()); // length
        vr.extend_from_slice(&0u16.to_be_bytes()); // checksum
        vr.extend_from_slice(&seg);

        b.extend_from_slice(&vr);
        b
    }

    #[test]
    fn parses_sul_and_frame_set() {
        let dlis = parse(&build_minimal_dlis()).unwrap();
        assert_eq!(dlis.label.structure, "RECORD");
        assert_eq!(dlis.label.set_identifier, b'S');
        assert_eq!(dlis.label.version, b'V');
        assert_eq!(dlis.logical_record_count, 1);
        assert_eq!(dlis.frames.len(), 1);
        assert_eq!(dlis.frames[0].name, "FR1");
        assert_eq!(dlis.frames[0].channels.len(), 1);
        assert_eq!(dlis.frames[0].channels[0].ident, "DEPT");
    }

    #[test]
    fn reads_fsingl_frame_data() {
        let mut bytes = build_minimal_dlis();
        // Append a visible record carrying an FSINGL data record (type 1):
        // body = frame-number (u16) + one f32 value per channel.
        let mut lr = Vec::new();
        lr.extend_from_slice(&1u16.to_be_bytes()); // frame number
        lr.extend_from_slice(&100.0f32.to_be_bytes()); // value (DEPT=100)
        let mut seg = Vec::new();
        seg.extend_from_slice(&(lr.len() as u16).to_be_bytes()); // segment length
        seg.push(1); // segment type = FSINGL
        seg.push(0); // attributes
        seg.extend_from_slice(&lr);
        let mut vr = Vec::new();
        vr.extend_from_slice(&(seg.len() as u16).to_be_bytes());
        vr.extend_from_slice(&0u16.to_be_bytes());
        vr.extend_from_slice(&seg);
        bytes.extend_from_slice(&vr);

        let dlis = parse(&bytes).unwrap();
        let frame = &dlis.frames[0];
        assert_eq!(frame.samples.len(), 1);
        assert_eq!(frame.samples[0].frame_number, 1);
        assert!((frame.samples[0].values[0] - 100.0).abs() < 1e-3);
    }
}
