//! ROS 1 bag files, format version 2.0: reader and writer.
//!
//! A bag starts with the line `#ROSBAG V2.0`, followed by length-prefixed records. Each record
//! has a header of `name=value` fields and an optional data section. Connections describe the
//! topics (with their message type and definition); messages reference a connection and carry a
//! timestamp and a serialized payload. Messages are normally stored inside chunks.
//!
//! ```
//! let bag = tpt_io_rosbag::Bag {
//!     connections: vec![tpt_io_rosbag::Connection {
//!         id: 0,
//!         topic: "/chatter".into(),
//!         msg_type: "std_msgs/String".into(),
//!         md5sum: "992ce8a1687cec8c8bd883ec73ca41d1".into(),
//!         definition: "string data".into(),
//!     }],
//!     messages: vec![tpt_io_rosbag::Message { connection: 0, time_ns: 1_000_000_123, data: vec![4, 0, 0, 0, b'h', b'i', b'!', b'!'] }],
//! };
//! let bytes = tpt_io_rosbag::write(&bag);
//! assert_eq!(tpt_io_rosbag::parse(&bytes).unwrap(), bag);
//! ```
//!
//! Only `none` chunk compression is read. `bz2` and `lz4` chunks return
//! [`RosbagError::Unsupported`]. Bags in the older version 1.2 layout are not read.
//!
//! Spec access: the ROS bag format is documented in the ROS wiki (BSD licence). No wiki text is
//! copied into this crate.

#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]

#[cfg(any(feature = "alloc", feature = "std"))]
#[allow(unused_imports)]
#[macro_use]
extern crate alloc;

pub mod error;
pub mod format;

pub use error::RosbagError;
pub use format::{parse, write, Bag, Connection, Message};
