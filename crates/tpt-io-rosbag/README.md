# tpt-io-rosbag

A reader and writer for ROS 1 bag files, format version 2.0 (`.bag`).

## Spec access and licence

The bag format is described on the ROS wiki, which is BSD-licensed. No wiki text is
copied into this crate. The layout here follows the published description of the
version 2.0 record format; test bags are built by hand in the tests.

## What it reads

- The `#ROSBAG V2.0` line and the length-prefixed record layout.
- Connection records, both top-level and inside chunks: topic, message type, MD5 sum
  and message definition.
- Message records, with the timestamp split into seconds and nanoseconds and
  combined into `time_ns`.
- Chunks with `compression=none`. Bag header, chunk-info and index records are read
  past without decoding.

## What it writes

A bag header followed by one uncompressed chunk holding the connections and
messages, in order. Parsing the output gives back the same `Bag`.

## Limits

- `bz2` and `lz4` chunks return `RosbagError::Unsupported`.
- Version 1.2 bags are not read.
- The index is not used, so reading is a single sequential pass.
- Message payloads are not deserialized; they are returned as bytes for the
  message type named in the connection.

## Example

```rust
let bag = tpt_io_rosbag::parse(&std::fs::read("run.bag")?)?;
for m in &bag.messages {
    let conn = bag.connections.iter().find(|c| c.id == m.connection);
    println!("{:?} {} bytes at {} ns", conn.map(|c| &c.topic), m.data.len(), m.time_ns);
}
```
