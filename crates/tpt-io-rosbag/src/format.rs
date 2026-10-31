//! ROS bag v2.0 record layout: reading, writing and the in-memory model.

use alloc::string::{String, ToString};
use alloc::vec::Vec;

use tpt_io_core::ParseError;

use crate::error::RosbagError;

const VERSION_LINE: &[u8] = b"#ROSBAG V2.0\n";

const OP_MSG_DATA: u8 = 0x02;
const OP_BAG_HEADER: u8 = 0x03;
const OP_INDEX_DATA: u8 = 0x04;
const OP_CHUNK: u8 = 0x05;
const OP_CHUNK_INFO: u8 = 0x06;
const OP_CONNECTION: u8 = 0x07;

/// A topic connection: its id, topic name and message type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Connection {
    /// Connection id, referenced by messages.
    pub id: u32,
    /// Topic name, e.g. `/chatter`.
    pub topic: String,
    /// Message type, e.g. `std_msgs/String`.
    pub msg_type: String,
    /// MD5 sum of the message definition.
    pub md5sum: String,
    /// The message definition text.
    pub definition: String,
}

/// One serialized message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Message {
    /// The connection this message was published on.
    pub connection: u32,
    /// Timestamp in nanoseconds since the epoch (from the `time` field's sec and nsec).
    pub time_ns: u64,
    /// The serialized payload.
    pub data: Vec<u8>,
}

/// A bag's connections and messages in file order.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Bag {
    /// Connections, in file order.
    pub connections: Vec<Connection>,
    /// Messages, in file order.
    pub messages: Vec<Message>,
}

/// One header field: `name=value` with a binary-safe value.
type Fields<'a> = Vec<(&'a [u8], &'a [u8])>;

struct Cursor<'a> {
    buf: &'a [u8],
    pos: usize,
}

impl<'a> Cursor<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8], ParseError> {
        let end = self.pos.checked_add(n).ok_or_else(|| ParseError::eof(self.pos))?;
        let s = self.buf.get(self.pos..end).ok_or_else(|| ParseError::eof(self.pos))?;
        self.pos = end;
        Ok(s)
    }

    fn u32(&mut self) -> Result<u32, ParseError> {
        let s = self.take(4)?;
        Ok(u32::from_le_bytes([s[0], s[1], s[2], s[3]]))
    }

    fn at_end(&self) -> bool {
        self.pos >= self.buf.len()
    }

    /// One length-prefixed record: header fields and data.
    fn record(&mut self) -> Result<(Fields<'a>, &'a [u8]), ParseError> {
        let hlen = self.u32()? as usize;
        let header = self.take(hlen)?;
        let dlen = self.u32()? as usize;
        let data = self.take(dlen)?;
        Ok((fields(header)?, data))
    }
}

fn fields(header: &[u8]) -> Result<Fields<'_>, ParseError> {
    let mut out = Vec::new();
    let mut pos = 0usize;
    while pos < header.len() {
        if pos + 4 > header.len() {
            return Err(ParseError::eof(pos));
        }
        let n = u32::from_le_bytes([header[pos], header[pos + 1], header[pos + 2], header[pos + 3]]) as usize;
        pos += 4;
        let field = header.get(pos..pos + n).ok_or_else(|| ParseError::eof(pos))?;
        pos += n;
        let eq = field
            .iter()
            .position(|&b| b == b'=')
            .ok_or_else(|| ParseError::custom(pos, "header field without '='"))?;
        out.push((&field[..eq], &field[eq + 1..]));
    }
    Ok(out)
}

fn get<'a>(fs: &Fields<'a>, name: &str) -> Option<&'a [u8]> {
    fs.iter().find(|(k, _)| *k == name.as_bytes()).map(|(_, v)| *v)
}

fn get_str(fs: &Fields<'_>, name: &str, at: usize) -> Result<String, ParseError> {
    let v = get(fs, name).ok_or_else(|| ParseError::custom(at, "missing header field"))?;
    core::str::from_utf8(v)
        .map(|s| s.to_string())
        .map_err(|_| ParseError::custom(at, "header field is not UTF-8"))
}

fn get_u32(fs: &Fields<'_>, name: &str, at: usize) -> Result<u32, ParseError> {
    let v = get(fs, name).ok_or_else(|| ParseError::custom(at, "missing header field"))?;
    let b: [u8; 4] = v.try_into().map_err(|_| ParseError::custom(at, "expected a 4-byte integer"))?;
    Ok(u32::from_le_bytes(b))
}

/// Parse a ROS 1 bag (version 2.0).
pub fn parse(input: &[u8]) -> Result<Bag, RosbagError> {
    if !input.starts_with(VERSION_LINE) {
        return Err(ParseError::custom(0, "missing '#ROSBAG V2.0' line").into());
    }
    let mut cur = Cursor { buf: input, pos: VERSION_LINE.len() };
    let mut bag = Bag::default();
    while !cur.at_end() {
        let at = cur.pos;
        let (fs, data) = cur.record()?;
        let op = get(&fs, "op").and_then(|v| v.first().copied());
        match op {
            Some(OP_CHUNK) => read_chunk(&fs, data, &mut bag, at)?,
            Some(OP_CONNECTION) => bag.connections.push(connection(&fs, data, at)?),
            Some(OP_MSG_DATA) => bag.messages.push(message(&fs, data, at)?),
            Some(OP_BAG_HEADER | OP_INDEX_DATA | OP_CHUNK_INFO) => {}
            _ => return Err(ParseError::custom(at, "unknown or missing record op").into()),
        }
    }
    Ok(bag)
}

fn read_chunk(fs: &Fields<'_>, data: &[u8], bag: &mut Bag, at: usize) -> Result<(), RosbagError> {
    let compression = get_str(fs, "compression", at)?;
    match compression.as_str() {
        "none" => {}
        "bz2" => return Err(RosbagError::Unsupported("bz2 chunk compression")),
        "lz4" => return Err(RosbagError::Unsupported("lz4 chunk compression")),
        _ => return Err(ParseError::custom(at, "unknown chunk compression").into()),
    }
    let mut cur = Cursor { buf: data, pos: 0 };
    while !cur.at_end() {
        let rat = at + cur.pos;
        let (fs, rdata) = cur.record()?;
        match get(&fs, "op").and_then(|v| v.first().copied()) {
            Some(OP_CONNECTION) => bag.connections.push(connection(&fs, rdata, rat)?),
            Some(OP_MSG_DATA) => bag.messages.push(message(&fs, rdata, rat)?),
            _ => return Err(ParseError::custom(rat, "unexpected record inside chunk").into()),
        }
    }
    Ok(())
}

fn connection(fs: &Fields<'_>, data: &[u8], at: usize) -> Result<Connection, ParseError> {
    let id = get_u32(fs, "conn", at)?;
    let topic = get_str(fs, "topic", at)?;
    // The data section carries the connection header as its own field list.
    let inner = fields(data)?;
    Ok(Connection {
        id,
        topic,
        msg_type: get_str(&inner, "type", at)?,
        md5sum: get_str(&inner, "md5sum", at)?,
        definition: get_str(&inner, "message_definition", at).unwrap_or_default(),
    })
}

fn message(fs: &Fields<'_>, data: &[u8], at: usize) -> Result<Message, ParseError> {
    let connection = get_u32(fs, "conn", at)?;
    let time = get(fs, "time").ok_or_else(|| ParseError::custom(at, "message without time"))?;
    if time.len() != 8 {
        return Err(ParseError::custom(at, "time must be 8 bytes"));
    }
    let sec = u32::from_le_bytes([time[0], time[1], time[2], time[3]]) as u64;
    let nsec = u32::from_le_bytes([time[4], time[5], time[6], time[7]]) as u64;
    Ok(Message {
        connection,
        time_ns: sec * 1_000_000_000 + nsec,
        data: data.to_vec(),
    })
}

fn put_field(out: &mut Vec<u8>, name: &str, value: &[u8]) {
    let n = name.len() + 1 + value.len();
    out.extend_from_slice(&(n as u32).to_le_bytes());
    out.extend_from_slice(name.as_bytes());
    out.push(b'=');
    out.extend_from_slice(value);
}

fn put_record(out: &mut Vec<u8>, header: &[u8], data: &[u8]) {
    out.extend_from_slice(&(header.len() as u32).to_le_bytes());
    out.extend_from_slice(header);
    out.extend_from_slice(&(data.len() as u32).to_le_bytes());
    out.extend_from_slice(data);
}

/// Serialise a bag as a single uncompressed chunk, followed by the bag header.
pub fn write(bag: &Bag) -> Vec<u8> {
    let mut chunk = Vec::new();
    for c in &bag.connections {
        let mut header = Vec::new();
        put_field(&mut header, "op", &[OP_CONNECTION]);
        put_field(&mut header, "conn", &c.id.to_le_bytes());
        put_field(&mut header, "topic", c.topic.as_bytes());
        let mut data = Vec::new();
        put_field(&mut data, "topic", c.topic.as_bytes());
        put_field(&mut data, "type", c.msg_type.as_bytes());
        put_field(&mut data, "md5sum", c.md5sum.as_bytes());
        put_field(&mut data, "message_definition", c.definition.as_bytes());
        put_record(&mut chunk, &header, &data);
    }
    for m in &bag.messages {
        let mut header = Vec::new();
        put_field(&mut header, "op", &[OP_MSG_DATA]);
        put_field(&mut header, "conn", &m.connection.to_le_bytes());
        let sec = (m.time_ns / 1_000_000_000) as u32;
        let nsec = (m.time_ns % 1_000_000_000) as u32;
        let mut t = Vec::with_capacity(8);
        t.extend_from_slice(&sec.to_le_bytes());
        t.extend_from_slice(&nsec.to_le_bytes());
        put_field(&mut header, "time", &t);
        put_record(&mut chunk, &header, &m.data);
    }

    let mut out = VERSION_LINE.to_vec();
    let mut bag_header = Vec::new();
    put_field(&mut bag_header, "op", &[OP_BAG_HEADER]);
    put_field(&mut bag_header, "index_pos", &0u64.to_le_bytes());
    put_field(&mut bag_header, "conn_count", &(bag.connections.len() as u32).to_le_bytes());
    put_field(&mut bag_header, "chunk_count", &1u32.to_le_bytes());
    put_record(&mut out, &bag_header, &[]);

    let mut chunk_header = Vec::new();
    put_field(&mut chunk_header, "op", &[OP_CHUNK]);
    put_field(&mut chunk_header, "compression", b"none");
    put_field(&mut chunk_header, "size", &(chunk.len() as u32).to_le_bytes());
    put_record(&mut out, &chunk_header, &chunk);
    out
}
