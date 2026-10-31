# tpt-io-mcap

A reader and writer for MCAP (`.mcap`) robotics log files, the container format
for timestamped messages used by ROS 2 and other robotics tooling.

## Spec access and licence

The MCAP file format is published by Foxglove under the Apache-2.0 licence. This
crate follows the published record layout (magic, header, schema, channel, message,
chunk, data-end, footer). No specification text is copied into the crate, and the
test files are built by hand in the tests.

## What it reads

- Header profile and library strings.
- Schemas, channels (with metadata) and messages, in file order.
- Messages stored directly in the data section and inside **uncompressed** chunks.
- Index, summary and attachment records are skipped; they are not needed to recover
  messages.

## What it writes

Header, schemas, channels, messages (unchunked), data-end and footer. Parsing the
output gives back the same `Mcap` value.

## Limits

- `lz4` and `zstd` chunks return `McapError::Unsupported`.
- Chunk CRCs and the data-section CRC are not checked.
- Attachments are skipped and not written.
- Reading whole files into memory; there is no streaming iterator yet.

## Example

```rust
let mcap = tpt_io_mcap::parse(&std::fs::read("run.mcap")?)?;
for msg in &mcap.messages {
    let topic = mcap.channels.iter().find(|c| c.id == msg.channel_id).map(|c| c.topic.as_str());
    println!("{topic:?} @ {}", msg.log_time);
}
```

`channel_id` matches a channel's `id` field, not its position in `channels`.
