//! Minimal ZIP archive reader: lists entries from the central directory and returns their decompressed bytes.
//!
//! Only the stored (method 0) and deflate (method 8) methods are supported, which covers the ZIP containers
//! used by OOXML-style and OMEX-style formats. ZIP64 and encryption are rejected.

use alloc::string::{String, ToString};
use alloc::vec::Vec;

use crate::error::{ParseError, Result};
use crate::inflate::inflate;

const EOCD: u32 = 0x0605_4b50;
const CENTRAL: u32 = 0x0201_4b50;
const LOCAL: u32 = 0x0403_4b50;

/// One archive member: its path and decompressed contents.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    /// The member path inside the archive, e.g. `3D/3dmodel.model`.
    pub name: String,
    /// The decompressed bytes.
    pub data: Vec<u8>,
}

fn u16_at(b: &[u8], at: usize) -> Result<u16> {
    let s = b.get(at..at + 2).ok_or_else(|| ParseError::eof(at))?;
    Ok(u16::from_le_bytes([s[0], s[1]]))
}

fn u32_at(b: &[u8], at: usize) -> Result<u32> {
    let s = b.get(at..at + 4).ok_or_else(|| ParseError::eof(at))?;
    Ok(u32::from_le_bytes([s[0], s[1], s[2], s[3]]))
}

/// Read every member of a ZIP archive, in central-directory order.
pub fn entries(input: &[u8]) -> Result<Vec<Entry>> {
    // The end-of-central-directory record is the last one in the file; scan back over a possible comment.
    let search_from = input.len().saturating_sub(22 + 0xFFFF);
    let eocd = (search_from..input.len().saturating_sub(21))
        .rev()
        .find(|&i| u32_at(input, i).ok() == Some(EOCD))
        .ok_or_else(|| ParseError::custom(0, "no ZIP end-of-central-directory record"))?;
    let count = u16_at(input, eocd + 10)? as usize;
    let mut pos = u32_at(input, eocd + 16)? as usize;
    let mut out = Vec::with_capacity(count);
    for _ in 0..count {
        if u32_at(input, pos)? != CENTRAL {
            return Err(ParseError::custom(pos, "bad central directory entry"));
        }
        let method = u16_at(input, pos + 10)?;
        let comp_size = u32_at(input, pos + 20)? as usize;
        let name_len = u16_at(input, pos + 28)? as usize;
        let extra_len = u16_at(input, pos + 30)? as usize;
        let comment_len = u16_at(input, pos + 32)? as usize;
        let local = u32_at(input, pos + 42)? as usize;
        let name_bytes =
            input.get(pos + 46..pos + 46 + name_len).ok_or_else(|| ParseError::eof(pos))?;
        let name = core::str::from_utf8(name_bytes)
            .map_err(|_| ParseError::custom(pos, "member name is not UTF-8"))?
            .to_string();
        if u32_at(input, local)? != LOCAL {
            return Err(ParseError::custom(local, "bad local file header"));
        }
        let local_name = u16_at(input, local + 26)? as usize;
        let local_extra = u16_at(input, local + 28)? as usize;
        let data_start = local + 30 + local_name + local_extra;
        let raw = input
            .get(data_start..data_start + comp_size)
            .ok_or_else(|| ParseError::eof(data_start))?;
        let data = match method {
            0 => raw.to_vec(),
            8 => inflate(raw)?,
            _ => {
                return Err(ParseError::unsupported(
                    pos,
                    "ZIP compression method other than stored or deflate",
                ))
            }
        };
        out.push(Entry { name, data });
        pos += 46 + name_len + extra_len + comment_len;
    }
    Ok(out)
}

/// Build a ZIP archive holding the given members, stored without compression. Useful for tests and for
/// writing small containers; the output reads back through [`entries`].
pub fn write_stored(members: &[(&str, &[u8])]) -> Vec<u8> {
    let mut z = Vec::new();
    let mut central = Vec::new();
    for (name, data) in members {
        let offset = z.len() as u32;
        let crc = crate::checksum::crc32(data);
        // Local file header: version, flags, method 0, time, date, crc, sizes, name and extra lengths.
        z.extend_from_slice(&LOCAL.to_le_bytes());
        z.extend_from_slice(&[20, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        z.extend_from_slice(&crc.to_le_bytes());
        z.extend_from_slice(&(data.len() as u32).to_le_bytes());
        z.extend_from_slice(&(data.len() as u32).to_le_bytes());
        z.extend_from_slice(&(name.len() as u16).to_le_bytes());
        z.extend_from_slice(&[0, 0]);
        z.extend_from_slice(name.as_bytes());
        z.extend_from_slice(data);
        // Central directory header for the same member.
        central.extend_from_slice(&CENTRAL.to_le_bytes());
        central.extend_from_slice(&[20, 0, 20, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        central.extend_from_slice(&crc.to_le_bytes());
        central.extend_from_slice(&(data.len() as u32).to_le_bytes());
        central.extend_from_slice(&(data.len() as u32).to_le_bytes());
        central.extend_from_slice(&(name.len() as u16).to_le_bytes());
        central.extend_from_slice(&[0; 12]);
        central.extend_from_slice(&offset.to_le_bytes());
        central.extend_from_slice(name.as_bytes());
    }
    let cd_offset = z.len() as u32;
    let cd_len = central.len() as u32;
    z.extend_from_slice(&central);
    z.extend_from_slice(&EOCD.to_le_bytes());
    z.extend_from_slice(&[0, 0, 0, 0]);
    z.extend_from_slice(&(members.len() as u16).to_le_bytes());
    z.extend_from_slice(&(members.len() as u16).to_le_bytes());
    z.extend_from_slice(&cd_len.to_le_bytes());
    z.extend_from_slice(&cd_offset.to_le_bytes());
    z.extend_from_slice(&[0, 0]);
    z
}

/// Find one member by path.
pub fn entry(input: &[u8], name: &str) -> Result<Option<Vec<u8>>> {
    Ok(entries(input)?.into_iter().find(|e| e.name == name).map(|e| e.data))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A one-member stored ZIP archive built by hand: "hello.txt" containing "hi".
    fn stored_zip() -> Vec<u8> {
        let name = b"hello.txt";
        let data = b"hi";
        let mut z = Vec::new();
        // Local file header.
        z.extend_from_slice(&LOCAL.to_le_bytes());
        z.extend_from_slice(&[20, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        z.extend_from_slice(&(data.len() as u32).to_le_bytes());
        z.extend_from_slice(&(data.len() as u32).to_le_bytes());
        z.extend_from_slice(&(name.len() as u16).to_le_bytes());
        z.extend_from_slice(&[0, 0]);
        z.extend_from_slice(name);
        z.extend_from_slice(data);
        let cd = z.len();
        // Central directory header.
        z.extend_from_slice(&CENTRAL.to_le_bytes());
        z.extend_from_slice(&[20, 0, 20, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        z.extend_from_slice(&(data.len() as u32).to_le_bytes());
        z.extend_from_slice(&(data.len() as u32).to_le_bytes());
        z.extend_from_slice(&(name.len() as u16).to_le_bytes());
        z.extend_from_slice(&[0; 12]);
        z.extend_from_slice(&0u32.to_le_bytes());
        z.extend_from_slice(name);
        let cd_len = z.len() - cd;
        // End of central directory.
        z.extend_from_slice(&EOCD.to_le_bytes());
        z.extend_from_slice(&[0, 0, 0, 0, 1, 0, 1, 0]);
        z.extend_from_slice(&(cd_len as u32).to_le_bytes());
        z.extend_from_slice(&(cd as u32).to_le_bytes());
        z.extend_from_slice(&[0, 0]);
        z
    }

    #[test]
    fn reads_a_stored_member() {
        let zip = stored_zip();
        let all = entries(&zip).unwrap();
        assert_eq!(all[0].name, "hello.txt");
        assert_eq!(all[0].data, b"hi");
        assert_eq!(entry(&zip, "missing").unwrap(), None);
    }

    #[test]
    fn non_zip_input_is_an_error() {
        assert!(entries(b"not a zip at all, just text").is_err());
    }
}
