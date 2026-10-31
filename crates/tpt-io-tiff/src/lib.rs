//! TIFF images: reader and writer for baseline strips, both byte orders, and IFD chains.
//!
//! A TIFF file starts with a byte-order mark (`II` or `MM`), the magic number 42 and the offset of the
//! first image file directory (IFD). Each IFD lists tagged entries (image size, sample format, strip
//! offsets and so on). This crate reads every IFD in the chain, decodes strips stored uncompressed or with
//! LZW compression, and exposes the pixels as little-endian bytes with their sample type. It writes
//! uncompressed single-strip images.
//!
//! ```
//! let image = tpt_io_tiff::Image {
//!     width: 2, height: 2, bits_per_sample: 8, samples_per_pixel: 1, sample_format: 1,
//!     description: Some("demo".into()),
//!     pixels: vec![1, 2, 3, 4],
//! };
//! let bytes = tpt_io_tiff::write(&[image.clone()]).unwrap();
//! let back = tpt_io_tiff::parse(&bytes).unwrap();
//! assert_eq!(back[0], image);
//! ```
//!
//! Tiled images, PackBits, Deflate, JPEG and BigTIFF are not decoded; they return
//! [`TiffError::Unsupported`]. CCITT and other fax compressions are not supported.
//!
//! Spec access: TIFF 6.0 is published by Adobe and is openly documented. No specification text is copied
//! into this crate.

#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]

#[cfg(any(feature = "alloc", feature = "std"))]
#[allow(unused_imports)]
#[macro_use]
extern crate alloc;

pub mod decode;
pub mod error;
pub mod tiff;

pub use error::TiffError;
pub use tiff::{parse, write, Image};
