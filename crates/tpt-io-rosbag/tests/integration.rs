//! Integration tests for the ROS 1 bag reader and writer.

use tpt_io_rosbag::{parse, write, Bag, Connection, Message, RosbagError};

fn sample() -> Bag {
    Bag {
        connections: vec![
            Connection {
                id: 0,
                topic: "/chatter".into(),
                msg_type: "std_msgs/String".into(),
                md5sum: "992ce8a1687cec8c8bd883ec73ca41d1".into(),
                definition: "string data".into(),
            },
            Connection {
                id: 1,
                topic: "/odom".into(),
                msg_type: "nav_msgs/Odometry".into(),
                md5sum: "cd5e73d190d741a2f92e81eda573aca7".into(),
                definition: String::new(),
            },
        ],
        messages: vec![
            Message { connection: 0, time_ns: 1_500_000_000, data: b"\x02\x00\x00\x00hi".to_vec() },
            Message { connection: 1, time_ns: 1_500_000_250, data: vec![0, 1, 2, 3] },
            Message { connection: 0, time_ns: 2_000_000_001, data: vec![] },
        ],
    }
}

#[test]
fn round_trip_keeps_connections_and_messages() {
    let bag = sample();
    let bytes = write(&bag);
    assert!(bytes.starts_with(b"#ROSBAG V2.0\n"));
    assert_eq!(parse(&bytes).unwrap(), bag);
}

#[test]
fn timestamps_split_into_seconds_and_nanoseconds() {
    let bytes = write(&sample());
    let bag = parse(&bytes).unwrap();
    assert_eq!(bag.messages[0].time_ns, 1_500_000_000);
    assert_eq!(bag.messages[2].time_ns, 2_000_000_001);
}

#[test]
fn compressed_chunks_are_unsupported() {
    let mut bytes = b"#ROSBAG V2.0\n".to_vec();
    let mut header = Vec::new();
    let op = b"op=\x05";
    header.extend_from_slice(&(op.len() as u32).to_le_bytes());
    header.extend_from_slice(op);
    let comp = b"compression=bz2";
    header.extend_from_slice(&(comp.len() as u32).to_le_bytes());
    header.extend_from_slice(comp);
    bytes.extend_from_slice(&(header.len() as u32).to_le_bytes());
    bytes.extend_from_slice(&header);
    bytes.extend_from_slice(&0u32.to_le_bytes());
    assert_eq!(parse(&bytes), Err(RosbagError::Unsupported("bz2 chunk compression")));
}

#[test]
fn wrong_version_and_truncation_are_errors() {
    assert!(parse(b"#ROSBAG V1.2\n").is_err());
    let full = write(&sample());
    for cut in [5usize, 20, full.len() - 3] {
        assert!(parse(&full[..cut]).is_err());
    }
}

#[test]
fn arbitrary_bytes_never_panic() {
    let mut x: u64 = 11;
    for _ in 0..400 {
        let len = (x % 200) as usize;
        let mut bytes: Vec<u8> = b"#ROSBAG V2.0\n".to_vec();
        bytes.extend((0..len).map(|_| {
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            x as u8
        }));
        let _ = parse(&bytes);
    }
}
