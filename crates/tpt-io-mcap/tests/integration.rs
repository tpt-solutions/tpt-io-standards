//! Integration tests for MCAP: a hand-built chunked file, round trips, and corruption.

use tpt_io_mcap::{parse, write, Channel, Mcap, McapError, Message, Schema};

/// Build a file by hand with one chunk holding a schema, a channel and two messages.
fn chunked_file() -> Vec<u8> {
    let mut f = Vec::new();
    f.extend_from_slice(b"\x89MCAP0\r\n");
    let mut header = Vec::new();
    header.extend_from_slice(&4u32.to_le_bytes());
    header.extend_from_slice(b"ros2");
    header.extend_from_slice(&0u32.to_le_bytes());
    record(&mut f, 0x01, &header);

    let mut inner = Vec::new();
    let mut schema = Vec::new();
    schema.extend_from_slice(&1u16.to_le_bytes());
    string(&mut schema, "std_msgs/String");
    string(&mut schema, "ros2msg");
    schema.extend_from_slice(&2u32.to_le_bytes());
    schema.extend_from_slice(b"ab");
    record_into(&mut inner, 0x03, &schema);

    let mut channel = Vec::new();
    channel.extend_from_slice(&7u16.to_le_bytes());
    channel.extend_from_slice(&1u16.to_le_bytes());
    string(&mut channel, "/chatter");
    string(&mut channel, "cdr");
    channel.extend_from_slice(&0u32.to_le_bytes());
    record_into(&mut inner, 0x04, &channel);

    for (seq, t, payload) in [(1u32, 100u64, &b"hello"[..]), (2, 200, &b"world"[..])] {
        let mut msg = Vec::new();
        msg.extend_from_slice(&7u16.to_le_bytes());
        msg.extend_from_slice(&seq.to_le_bytes());
        msg.extend_from_slice(&t.to_le_bytes());
        msg.extend_from_slice(&t.to_le_bytes());
        msg.extend_from_slice(payload);
        record_into(&mut inner, 0x05, &msg);
    }

    let mut chunk = Vec::new();
    chunk.extend_from_slice(&100u64.to_le_bytes());
    chunk.extend_from_slice(&200u64.to_le_bytes());
    chunk.extend_from_slice(&(inner.len() as u64).to_le_bytes());
    chunk.extend_from_slice(&0u32.to_le_bytes());
    string(&mut chunk, "");
    chunk.extend_from_slice(&(inner.len() as u64).to_le_bytes());
    chunk.extend_from_slice(&inner);
    record(&mut f, 0x06, &chunk);

    record(&mut f, 0x02, &[0u8; 20]);
    f.extend_from_slice(b"\x89MCAP0\r\n");
    f
}

fn record(out: &mut Vec<u8>, op: u8, body: &[u8]) {
    record_into(out, op, body);
}

fn record_into(out: &mut Vec<u8>, op: u8, body: &[u8]) {
    out.push(op);
    out.extend_from_slice(&(body.len() as u64).to_le_bytes());
    out.extend_from_slice(body);
}

fn string(out: &mut Vec<u8>, s: &str) {
    out.extend_from_slice(&(s.len() as u32).to_le_bytes());
    out.extend_from_slice(s.as_bytes());
}

#[test]
fn reads_messages_from_chunks() {
    let m = parse(&chunked_file()).unwrap();
    assert_eq!(m.profile, "ros2");
    assert_eq!(m.schemas[0].name, "std_msgs/String");
    assert_eq!(m.schemas[0].data, b"ab");
    assert_eq!(m.channels[0].topic, "/chatter");
    assert_eq!(m.messages.len(), 2);
    assert_eq!(m.messages[1].log_time, 200);
    assert_eq!(m.messages[0].data, b"hello");
}

#[test]
fn write_then_parse_is_identity() {
    let m = parse(&chunked_file()).unwrap();
    let bytes = write(&m);
    assert_eq!(parse(&bytes).unwrap(), m);
}

#[test]
fn metadata_and_empty_file_round_trip() {
    let m = Mcap {
        profile: String::new(),
        library: "t".into(),
        schemas: vec![Schema { id: 0, name: String::new(), encoding: String::new(), data: vec![] }],
        channels: vec![Channel {
            id: 3,
            schema_id: 0,
            topic: "/t".into(),
            message_encoding: "json".into(),
            metadata: vec![("k".into(), "v".into()), ("".into(), "x".into())],
        }],
        messages: vec![Message {
            channel_id: 3,
            sequence: 0,
            log_time: 0,
            publish_time: 0,
            data: vec![],
        }],
    };
    assert_eq!(parse(&write(&m)).unwrap(), m);

    let empty = Mcap {
        profile: String::new(),
        library: String::new(),
        schemas: vec![],
        channels: vec![],
        messages: vec![],
    };
    assert_eq!(parse(&write(&empty)).unwrap(), empty);
}

#[test]
fn compressed_chunks_are_reported_as_unsupported() {
    let mut g = Vec::new();
    g.extend_from_slice(b"\x89MCAP0\r\n");
    let mut chunk = Vec::new();
    chunk.extend_from_slice(&0u64.to_le_bytes());
    chunk.extend_from_slice(&0u64.to_le_bytes());
    chunk.extend_from_slice(&0u64.to_le_bytes());
    chunk.extend_from_slice(&0u32.to_le_bytes());
    string(&mut chunk, "zstd");
    chunk.extend_from_slice(&0u64.to_le_bytes());
    record_into(&mut g, 0x06, &chunk);
    g.extend_from_slice(b"\x89MCAP0\r\n");
    assert_eq!(parse(&g), Err(McapError::Unsupported("zstd chunk compression")));
}

#[test]
fn truncation_and_bad_magic_are_errors() {
    let full = chunked_file();
    for cut in [0usize, 5, 20, full.len() / 2] {
        assert!(parse(&full[..cut]).is_err());
    }
    let mut bad = full.clone();
    bad[0] = 0;
    assert!(parse(&bad).is_err());
}

#[test]
fn arbitrary_bytes_never_panic() {
    let mut x: u64 = 3;
    for _ in 0..500 {
        let len = (x % 300) as usize;
        let mut bytes: Vec<u8> = (0..len)
            .map(|_| {
                x ^= x << 13;
                x ^= x >> 7;
                x ^= x << 17;
                x as u8
            })
            .collect();
        if bytes.len() >= 8 {
            bytes[..8].copy_from_slice(b"\x89MCAP0\r\n");
        }
        let _ = parse(&bytes);
    }
}
