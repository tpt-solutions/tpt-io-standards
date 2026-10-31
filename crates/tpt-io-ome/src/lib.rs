//! OME microscopy metadata: OME-XML from OME-TIFF files, and OME-Zarr multiscale and array metadata.
//!
//! OME-TIFF stores the image's OME-XML in the `ImageDescription` tag of its first IFD. The XML's `Pixels`
//! element gives the dimension order, the sizes (X, Y, Z, C, T) and the pixel type. OME-Zarr stores the same
//! information as JSON: `.zattrs` lists the multiscale datasets and their axes, and each array's `.zarray`
//! gives its shape, chunking and data type. This crate reads both forms.
//!
//! ```
//! let xml = br#"<OME><Image ID="Image:0"><Pixels ID="Pixels:0" DimensionOrder="XYZCT" Type="uint16"
//!   SizeX="512" SizeY="512" SizeZ="1" SizeC="2" SizeT="1"/></Image></OME>"#;
//! let image = tpt_io_ome::parse_xml(xml).unwrap();
//! assert_eq!(image.size_c, 2);
//! assert_eq!(image.pixel_type, "uint16");
//! ```
//!
//! Only the first image and its first `Pixels` element are read. Pixel data is read by `tpt-io-tiff`.
//!
//! Spec access: OME-XML and OME-Zarr are open specifications published by the Open Microscopy Environment.
//! No specification text is copied into this crate.

#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]

#[cfg(any(feature = "alloc", feature = "std"))]
#[allow(unused_imports)]
#[macro_use]
extern crate alloc;

pub mod error;
pub mod ome_xml;
pub mod zarr;

pub use error::OmeError;
pub use ome_xml::{from_tiff, parse_xml, Pixels};
pub use zarr::{parse_zarray, parse_zattrs, ZArray, Multiscales};
