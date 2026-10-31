//! ISO/IEC 8211 transfer framing, as used by S-57: leaders, directories, field areas and the
//! data descriptive record (DDR) that gives the format of every later field.
//!
//! A file is a sequence of records. Each record has a 24-byte leader, a directory of
//! `tag | length | position` entries ended by a field terminator, and a field area. The first
//! record is the DDR. Its fields name each field's subfields and give their formats, so
//! [`dictionary`] and [`decode`] can read the data records that follow.
//!
//! Binary subfields are read little-endian. Fixed-width subfields have no terminator; variable
//! ones end with a unit terminator, and the last subfield of a field ends with the field
//! terminator, which is stripped from the field data.

use alloc::{collections::BTreeMap, format, string::String, vec::Vec};

use tpt_io_core::ParseError;

/// Ends each field in the field area, and the directory.
pub const FIELD_TERMINATOR: u8 = 0x1E;
/// Follows each variable-length subfield that is not the last one in its field.
pub const UNIT_TERMINATOR: u8 = 0x1F;
/// Ends each record.
pub const RECORD_TERMINATOR: u8 = 0x1D;

const LEADER_LEN: usize = 24;
const FIELD_CONTROL_LEN: usize = 9;

/// One elementary format control from a DDR field description.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Control {
    /// Character data: `A(n)` has a fixed width, `A` runs to the unit terminator.
    Text(Option<usize>),
    /// ASCII numeric data: `I(n)` and `R(n)` behave like [`Control::Text`].
    Numeric(Option<usize>),
    /// A bit string of `n` bits, stored in `ceil(n / 8)` bytes: `B(n)`.
    Bits(usize),
    /// A binary integer of `width` bytes: `b1n` is unsigned, `b2n` signed.
    Binary { width: usize, signed: bool },
}

/// The tag, subfield labels and format controls of one field, from the DDR.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldDef {
    /// The directory tag, for example `FRID`.
    pub tag: String,
    /// Subfield labels, cycled over the format controls.
    pub labels: Vec<String>,
    /// Elementary format controls, cycled until the field data is used up.
    pub controls: Vec<Control>,
}

/// Field definitions keyed by tag.
pub type Dictionary = BTreeMap<String, FieldDef>;

/// A decoded subfield value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value {
    /// Character data (also the ASCII numeric formats).
    Text(String),
    /// An unsigned binary integer (`b1n`).
    Unsigned(u64),
    /// A signed binary integer (`b2n`).
    Signed(i64),
    /// The raw bytes of a bit string (`B(n)`), in stream order.
    Bytes(Vec<u8>),
}

/// One decoded subfield: its label from the DDR and its value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Subfield {
    /// The DDR subfield label, or empty when the DDR gives none.
    pub label: String,
    /// The decoded value.
    pub value: Value,
}

/// One field of a record. `data` excludes the field terminator.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Field {
    /// The directory tag.
    pub tag: String,
    /// The field's raw data.
    pub data: Vec<u8>,
}

/// One ISO 8211 record.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Record {
    /// The fields in directory order.
    pub fields: Vec<Field>,
}

/// Reads every record in a transfer file. The first record is the DDR.
pub fn read_records(input: &[u8]) -> Result<Vec<Record>, ParseError> {
    let mut records = Vec::new();
    let mut start = 0;
    while start < input.len() {
        let (record, next) = read_record(input, start)?;
        records.push(record);
        start = next;
    }
    Ok(records)
}

fn read_record(input: &[u8], start: usize) -> Result<(Record, usize), ParseError> {
    let leader =
        input.get(start..start + LEADER_LEN).ok_or_else(|| ParseError::eof(input.len()))?;
    let record_len = number(&leader[0..5], start)?;
    let base = number(&leader[12..17], start + 12)?;
    let len_size = number(&leader[20..21], start + 20)?;
    let pos_size = number(&leader[21..22], start + 21)?;
    let tag_size = number(&leader[23..24], start + 23)?;

    if record_len <= LEADER_LEN || record_len > input.len() - start {
        return Err(ParseError::custom(start, "ISO 8211 record length out of range"));
    }
    if base > record_len || len_size == 0 || pos_size == 0 || tag_size == 0 {
        return Err(ParseError::custom(start, "ISO 8211 leader has an impossible layout"));
    }

    let record = &input[start..start + record_len];
    let last = record[record_len - 1];
    if last != RECORD_TERMINATOR {
        return Err(ParseError::invalid_byte(start + record_len - 1, last));
    }

    let entry_len = tag_size + len_size + pos_size;
    let mut entries = Vec::new();
    let mut at = LEADER_LEN;
    loop {
        let byte = *record.get(at).ok_or_else(|| ParseError::eof(start + at))?;
        if byte == FIELD_TERMINATOR {
            break;
        }
        let entry = record.get(at..at + entry_len).ok_or_else(|| ParseError::eof(start + at))?;
        let tag = core::str::from_utf8(&entry[..tag_size])
            .map_err(|_| ParseError::custom(start + at, "ISO 8211 field tag is not text"))?;
        let length = number(&entry[tag_size..tag_size + len_size], start + at + tag_size)?;
        let position = number(&entry[tag_size + len_size..], start + at + tag_size + len_size)?;
        entries.push((String::from(tag), length, position));
        at += entry_len;
    }

    let mut fields = Vec::with_capacity(entries.len());
    for (tag, length, position) in entries {
        let from = base
            .checked_add(position)
            .ok_or_else(|| ParseError::custom(start, "ISO 8211 field position overflows"))?;
        let to = from
            .checked_add(length)
            .ok_or_else(|| ParseError::custom(start, "ISO 8211 field length overflows"))?;
        let mut data = record.get(from..to).ok_or_else(|| ParseError::eof(start + from))?;
        if let Some((&FIELD_TERMINATOR, rest)) = data.split_last() {
            data = rest;
        }
        fields.push(Field { tag, data: data.to_vec() });
    }
    Ok((Record { fields }, start + record_len))
}

/// Parses the field definitions in a DDR. The `0000` entry, which describes the record itself,
/// is skipped.
pub fn dictionary(ddr: &Record) -> Result<Dictionary, ParseError> {
    let mut defs = BTreeMap::new();
    for field in &ddr.fields {
        if field.tag == "0000" {
            continue;
        }
        defs.insert(field.tag.clone(), field_def(&field.tag, &field.data)?);
    }
    Ok(defs)
}

fn field_def(tag: &str, data: &[u8]) -> Result<FieldDef, ParseError> {
    // The field control is nine bytes (structure, data type, auxiliary, printable and escape
    // controls). After it come the field name, an optional array descriptor and the format
    // controls, separated by unit terminators.
    let body = data
        .get(FIELD_CONTROL_LEN..)
        .ok_or_else(|| ParseError::custom(0, format!("field {tag} has no field control")))?;
    let parts: Vec<&[u8]> = body.split(|&b| b == UNIT_TERMINATOR).collect();
    if parts.len() < 2 {
        return Err(ParseError::custom(0, format!("field {tag} has no format controls")));
    }
    let name = parts[0];
    let format = parts[parts.len() - 1];
    let descriptor: &[u8] = if parts.len() >= 3 { parts[1] } else { &[] };

    let mut labels: Vec<String> = descriptor
        .split(|&b| b == b'!')
        .filter(|label| !label.is_empty())
        .map(|label| String::from_utf8_lossy(label).into_owned())
        .collect();
    if labels.is_empty() {
        labels.push(String::from_utf8_lossy(name).into_owned());
    }

    let controls = controls(format, tag)?;
    Ok(FieldDef { tag: String::from(tag), labels, controls })
}

/// Reads a format control string such as `(b11,b14,A,A(8))` into elementary controls. Group
/// brackets, commas, repeat counts and `*` are structure, not controls, and are skipped.
fn controls(format: &[u8], tag: &str) -> Result<Vec<Control>, ParseError> {
    let unknown = || ParseError::custom(0, format!("unsupported format control in field {tag}"));
    let mut out = Vec::new();
    let mut at = 0;
    while at < format.len() {
        let letter = format[at];
        at += 1;
        match letter {
            b'A' | b'I' | b'R' | b'B' => {
                let (count, next) = count_at(format, at).ok_or_else(unknown)?;
                at = next;
                out.push(match letter {
                    b'A' => Control::Text(count),
                    b'I' | b'R' => Control::Numeric(count),
                    _ => Control::Bits(count.ok_or_else(unknown)?),
                });
            }
            b'b' => {
                let class = *format.get(at).ok_or_else(unknown)?;
                let width = *format.get(at + 1).ok_or_else(unknown)?;
                at += 2;
                let signed = match class {
                    b'1' => false,
                    b'2' => true,
                    _ => return Err(unknown()),
                };
                let width = match width {
                    b'1'..=b'8' => usize::from(width - b'0'),
                    _ => return Err(unknown()),
                };
                out.push(Control::Binary { width, signed });
            }
            b'(' | b')' | b',' | b'*' | b'0'..=b'9' => {}
            _ => return Err(unknown()),
        }
    }
    Ok(out)
}

/// Reads an optional `(n)` width at `at`, returning it and the index after it.
fn count_at(format: &[u8], at: usize) -> Option<(Option<usize>, usize)> {
    if format.get(at) != Some(&b'(') {
        return Some((None, at));
    }
    let close = at + format[at..].iter().position(|&b| b == b')')?;
    let inner = &format[at + 1..close];
    if inner.is_empty() {
        return Some((None, close + 1));
    }
    let count = core::str::from_utf8(inner).ok()?.parse().ok()?;
    Some((Some(count), close + 1))
}

/// Decodes the data of one field against its definition, returning its subfields in order.
///
/// The controls and labels repeat until the data is used up, so a repeating group such as
/// `(b12,A)` decodes as a flat list of pairs.
pub fn decode(def: &FieldDef, data: &[u8]) -> Result<Vec<Subfield>, ParseError> {
    let mut out = Vec::new();
    if data.is_empty() {
        return Ok(out);
    }
    if def.controls.is_empty() {
        return Err(ParseError::custom(0, format!("field {} has no format controls", def.tag)));
    }
    let mut at = 0;
    let mut index = 0;
    while at < data.len() {
        let control = def.controls[index % def.controls.len()];
        let label = if def.labels.is_empty() {
            String::new()
        } else {
            def.labels[index % def.labels.len()].clone()
        };
        let (value, used) = take(control, &data[at..], at)?;
        if used == 0 {
            return Err(ParseError::custom(at, format!("zero-width subfield in {}", def.tag)));
        }
        out.push(Subfield { label, value });
        at += used;
        index += 1;
    }
    Ok(out)
}

fn take(control: Control, rest: &[u8], at: usize) -> Result<(Value, usize), ParseError> {
    match control {
        Control::Text(None) | Control::Numeric(None) => {
            let (text, used) = match rest.iter().position(|&b| b == UNIT_TERMINATOR) {
                Some(end) => (&rest[..end], end + 1),
                None => (rest, rest.len()),
            };
            Ok((Value::Text(String::from_utf8_lossy(text).into_owned()), used))
        }
        Control::Text(Some(width)) | Control::Numeric(Some(width)) => {
            let text = fixed(rest, width, at)?;
            Ok((Value::Text(String::from_utf8_lossy(text).into_owned()), width))
        }
        Control::Bits(bits) => {
            let len = bits.div_ceil(8);
            Ok((Value::Bytes(fixed(rest, len, at)?.to_vec()), len))
        }
        Control::Binary { width, signed } => {
            let bytes = fixed(rest, width, at)?;
            let raw =
                bytes.iter().enumerate().fold(0u64, |acc, (i, &b)| acc | (u64::from(b) << (8 * i)));
            let value = if signed {
                // Sign-extend from `width` bytes: move the top bit of the field to bit 63.
                let shift = 64 - 8 * width as u32;
                Value::Signed(((raw << shift) as i64) >> shift)
            } else {
                Value::Unsigned(raw)
            };
            Ok((value, width))
        }
    }
}

fn fixed(rest: &[u8], width: usize, at: usize) -> Result<&[u8], ParseError> {
    rest.get(..width).ok_or_else(|| ParseError::eof(at + rest.len()))
}

fn number(bytes: &[u8], offset: usize) -> Result<usize, ParseError> {
    let mut value = 0usize;
    for (i, &byte) in bytes.iter().enumerate() {
        if !byte.is_ascii_digit() {
            return Err(ParseError::invalid_byte(offset + i, byte));
        }
        value = value
            .checked_mul(10)
            .and_then(|v| v.checked_add(usize::from(byte - b'0')))
            .ok_or_else(|| ParseError::custom(offset, "ISO 8211 number overflows"))?;
    }
    Ok(value)
}
