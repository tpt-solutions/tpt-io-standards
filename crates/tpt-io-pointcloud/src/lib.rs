//! LAS point clouds (ASPRS LiDAR, `.las`): reader and writer for uncompressed files.
//!
//! A LAS file has a public header (version, scale factors, offsets, point count), optional variable-length
//! records, and a block of point records. Coordinates are stored as scaled integers: `x = raw * scale + offset`.
//! This crate reads point record formats 0 to 3 for LAS 1.0 to 1.4, and writes LAS 1.2 with format 0 or 1.
//! Compressed LAZ files are not read.
//!
//! ```
//! let mut cloud = tpt_io_pointcloud::Cloud::default();
//! cloud.scale = [0.01, 0.01, 0.01];
//! cloud.points.push(tpt_io_pointcloud::Point { position: [1.5, -2.0, 30.25], intensity: 120, classification: 2, ..Default::default() });
//! let bytes = tpt_io_pointcloud::write(&cloud).unwrap();
//! let back = tpt_io_pointcloud::parse(&bytes).unwrap();
//! assert_eq!(back.points[0].position, [1.5, -2.0, 30.25]);
//! ```
//!
//! Spec access: LAS is published by ASPRS (the American Society for Photogrammetry and Remote Sensing)
//! under its standard terms. No specification text is copied into this crate.

#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]

#[cfg(any(feature = "alloc", feature = "std"))]
#[allow(unused_imports)]
#[macro_use]
extern crate alloc;

pub mod error;
pub mod las;

pub use error::LasError;
pub use las::{parse, write, Cloud, Point};
