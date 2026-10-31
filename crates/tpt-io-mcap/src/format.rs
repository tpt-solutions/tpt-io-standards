//! MCAP record layout: reading, writing and the in-memory model.

use alloc::string::String;
use alloc::vec::Vec;

use tpt_io_core::ParseError;

use crate::error::McapError;

const MAGIC: &[u8; 8] = b"\x89MCAP0\r\n";

const OP_HEADER: u8 = 0x01;
const OP_FOOTER: u8 = 0x02;
const OP_SCHEMA: u8 = 0x03;
const OP_CHANNEL: u8 = 0x04;
const OP_MESSAGE: u8 = 0x05;
const OP_CHUNK: u8 = 0x06;
const OP_DATA_END: u8 = 0x0F;

/// A schema describing the layout of messages on one or more channels.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Schema {
    /// Schema id, referenced by channels.
    pub id: u16,
    /// Schema name, e.g. a ROS message type.
    pub name: String,
    /// Schema encoding, e.g. `"ros2msg"` or `"jsonschema"`.
    pub encoding: String,
    /// The schema definition bytes.
    pub data: Vec<u8>,
}

/// A named stream of messages.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Channel {
    /// Channel id, referenced by messages.
    pub id: u16,
    /// Schema id, or 0 when the channel has no schema.
    pub schema_id: u16,
    /// Topic name, e.g. `/chatter`.
    pub topic: String,
    /// Message encoding, e.g. `"cdr"` or `"json"`.
    pub message_encoding: String,
    /// Key/value metadata.
    pub metadata: Vec<(String, String)>,
}

/// One message on a channel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Message {
    /// The channel this message belongs to.
    pub channel_id: u16,
    /// Per-channel sequence number.
    pub sequence: u32,
    /// Log time, nanoseconds.
    pub log_time: u64,
    /// Publish time, nanoseconds.
    pub publish_time: u64,
    /// The encoded message payload.
    pub data: Vec<u8>,
}

/// An MCAP file's header and its schemas, channels and messages in file order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mcap {
    /// Header profile, e.g. `"ros2"`.
    pub profile: String,
    /// Header library string identifying the writer.
    pub library: String,
    /// Schemas in file order.
    pub schemas: Vec<Schema>,
    /// Channels in file order.
    pub channels: Vec<Channel>,
    /// Messages in file order (chunked and unchunked alike).
    pub messages: Vec<Message>,
}

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

    fn u8(&mut self) -> Result<u8, ParseError> {
        Ok(self.take(1)?[0])
    }

    fn u16(&mut self) -> Result<u16, ParseError> {
        let s = self.take(2)?;
        Ok(u16::from_le_bytes([s[0], s[1]]))
    }

    fn u32(&mut self) -> Result<u32, ParseError> {
        let s = self.take(4)?;
        Ok(u32::from_le_bytes([s[0], s[1], s[2], s[3]]))
    }

    fn u64(&mut self) -> Result<u64, ParseError> {
        let s = self.take(8)?;
        let mut b = [0u8; 8];
        b.copy_from_slice(s);
        Ok(u64::from_le_bytes(b))
    }

    fn string(&mut self) -> Result<String, ParseError> {
        let at = self.pos;
        let n = self.u32()? as usize;
        let s = self.take(n)?;
        core::str::from_utf8(s)
            .map(String::from)
            .map_err(|_| ParseError::custom(at, "string is not valid UTF-8"))
    }

    fn bytes_u32(&mut self) -> Result<Vec<u8>, ParseError> {
        let n = self.u32()? as usize;
        Ok(self.take(n)?.to_vec())
    }

    fn rest(&mut self) -> &'a [u8] {
        let s = &self.buf[self.pos..];
        self.pos = self.buf.len();
        s
    }

    fn map(&mut self) -> Result<Vec<(String, String)>, ParseError> {
        let n = self.u32()? as usize;
        let end = self.pos + n;
        let mut out = Vec::new();
        while self.pos < end {
            let k = self.string()?;
            let v = self.string()?;
            out.push((k, v));
        }
        if self.pos != end {
            return Err(ParseError::custom(self.pos, "metadata map overruns its length"));
        }
        Ok(out)
    }
}

/// Parse an MCAP file.
pub fn parse(input: &[u8]) -> Result<Mcap, McapError> {
    if input.len() < MAGIC.len() * 2 || &input[..8] != MAGIC || &input[input.len() - 8..] != MAGIC {
        return Err(ParseError::custom(0, "missing MCAP magic").into());
    }
    let body = &input[8..input.len() - 8];
    let mut out = Mcap {
        profile: String::new(),
        library: String::new(),
        schemas: Vec::new(),
        channels: Vec::new(),
        messages: Vec::new(),
    };
    let mut cur = Cursor { buf: body, pos: 0 };
    while cur.pos < body.len() {
        read_record(&mut cur, &mut out, 0)?;
    }
    Ok(out)
}

fn read_record(cur: &mut Cursor<'_>, out: &mut Mcap, base: usize) -> Result<(), McapError> {
    let at = base + cur.pos;
    let op = cur.u8()?;
    let len = cur.u64()? as usize;
    let data = cur.take(len).map_err(|_| ParseError::custom(at, "record length overruns the file"))?;
    let mut r = Cursor { buf: data, pos: 0 };
    match op {
        OP_HEADER => {
            out.profile = r.string()?;
            out.library = r.string()?;
        }
        OP_SCHEMA => out.schemas.push(Schema {
            id: r.u16()?,
            name: r.string()?,
            encoding: r.string()?,
            data: r.bytes_u32()?,
        }),
        OP_CHANNEL => out.channels.push(Channel {
            id: r.u16()?,
            schema_id: r.u16()?,
            topic: r.string()?,
            message_encoding: r.string()?,
            metadata: r.map()?,
        }),
        OP_MESSAGE => out.messages.push(message(&mut r)?),
        OP_CHUNK => read_chunk(&mut r, out, at)?,
        OP_FOOTER | OP_DATA_END => {}
        _ => {} // index and summary records are not needed to recover the messages
    }
    Ok(())
}

fn message(r: &mut Cursor<'_>) -> Result<Message, ParseError> {
    Ok(Message {
        channel_id: r.u16()?,
        sequence: r.u32()?,
        log_time: r.u64()?,
        publish_time: r.u64()?,
        data: r.rest().to_vec(),
    })
}

fn read_chunk(r: &mut Cursor<'_>, out: &mut Mcap, at: usize) -> Result<(), McapError> {
    let _message_start = r.u64()?;
    let _message_end = r.u64()?;
    let _uncompressed_size = r.u64()?;
    let _crc = r.u32()?;
    let compression = r.string()?;
    let records = r.bytes_u32_u64()?;
    match compression.as_str() {
        "" => {}
        "lz4" => return Err(McapError::Unsupported("lz4 chunk compression")),
        "zstd" => return Err(McapError::Unsupported("zstd chunk compression")),
        _ => return Err(ParseError::custom(at, "unknown chunk compression").into()),
    }
    let mut inner = Cursor { buf: records, pos: 0 };
    while inner.pos < records.len() {
        read_record(&mut inner, out, at)?;
    }
    Ok(())
}

impl<'a> Cursor<'a> {
    /// A `u64`-length-prefixed byte field (used for chunk record payloads).
    fn bytes_u32_u64(&mut self) -> Result<&'a [u8], ParseError> {
        let n = self.u64()? as usize;
        self.take(n)
    }
}

/// Serialise an MCAP file. Messages are written unchunked in file order.
pub fn write(m: &Mcap) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(MAGIC);

    let mut header = Vec::new();
    put_string(&mut header, &m.profile);
    put_string(&mut header, &m.library);
    put_record(&mut out, OP_HEADER, &header);

    for s in &m.schemas {
        let mut b = Vec::new();
        b.extend_from_slice(&s.id.to_le_bytes());
        put_string(&mut b, &s.name);
        put_string(&mut b, &s.encoding);
        put_bytes_u32(&mut b, &s.data);
        put_record(&mut out, OP_SCHEMA, &b);
    }
    for c in &m.channels {
        let mut b = Vec::new();
        b.extend_from_slice(&c.id.to_le_bytes());
        b.extend_from_slice(&c.schema_id.to_le_bytes());
        put_string(&mut b, &c.topic);
        put_string(&mut b, &c.message_encoding);
        let mut map = Vec::new();
        for (k, v) in &c.metadata {
            put_string(&mut map, k);
            put_string(&mut map, v);
        }
        put_bytes_u32(&mut b, &map);
        put_record(&mut out, OP_CHANNEL, &b);
    }
    for msg in &m.messages {
        let mut b = Vec::new();
        b.extend_from_slice(&msg.channel_id.to_le_bytes());
        b.extend_from_slice(&msg.sequence.to_le_bytes());
        b.extend_from_slice(&msg.log_time.to_le_bytes());
        b.extend_from_slice(&msg.publish_time.to_le_bytes());
        b.extend_from_slice(&msg.data);
        put_record(&mut out, OP_MESSAGE, &b);
    }

    put_record(&mut out, OP_DATA_END, &0u32.to_le_bytes());
    let mut footer = Vec::new();
    footer.extend_from_slice(&0u64.to_le_bytes());
    footer.extend_from_slice(&0u64.to_le_bytes());
    footer.extend_from_slice(&0u32.to_le_bytes());
    put_record(&mut out, OP_FOOTER, &footer);

    out.extend_from_slice(MAGIC);
    out
}

fn put_record(out: &mut Vec<u8>, op: u8, body: &[u8]) {
    out.push(op);
    out.extend_from_slice(&(body.len() as u64).to_le_bytes());
    out.extend_from_slice(body);
}

fn put_string(out: &mut Vec<u8>, s: &str) {
    out.extend_from_slice(&(s.len() as u32).to_le_bytes());
    out.extend_from_slice(s.as_bytes());
}

fn put_bytes_u32(out: &mut Vec<u8>, b: &[u8]) {
    out.extend_from_slice(&(b.len() as u32).to_le_bytes());
    out.extend_from_slice(b);
}
