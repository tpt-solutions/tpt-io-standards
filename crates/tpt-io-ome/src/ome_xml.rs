//! OME-XML `Pixels` reading, and OME-TIFF image description extraction.

use alloc::string::{String, ToString};

use tpt_io_core::xml;
use tpt_io_core::ParseError;

use crate::error::OmeError;

/// The `Pixels` element of the first image: sizes, dimension order and pixel type.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Pixels {
    /// The `Pixels/@ID`.
    pub id: String,
    /// The `DimensionOrder`, e.g. `XYZCT`.
    pub dimension_order: String,
    /// The pixel `Type`, e.g. `uint16`.
    pub pixel_type: String,
    /// Size along X.
    pub size_x: u32,
    /// Size along Y.
    pub size_y: u32,
    /// Size along Z.
    pub size_z: u32,
    /// Number of channels.
    pub size_c: u32,
    /// Number of time points.
    pub size_t: u32,
}

fn size(el: &xml::Element, name: &str) -> Result<u32, OmeError> {
    el.attr(name)
        .ok_or_else(|| OmeError::Invalid(alloc::format!("Pixels has no {name}")))?
        .parse::<u32>()
        .map_err(|_| OmeError::Core(ParseError::custom(el.offset, "size is not an integer")))
}

/// Parse OME-XML and return the first image's `Pixels`.
pub fn parse_xml(input: &[u8]) -> Result<Pixels, OmeError> {
    let root = xml::parse(input)?;
    if root.local_name() != "OME" {
        return Err(OmeError::Invalid("root element is not <OME>".to_string()));
    }
    let pixels = root
        .descendants_named("Pixels")
        .first()
        .copied()
        .ok_or_else(|| OmeError::Invalid("no Pixels element".to_string()))?;
    Ok(Pixels {
        id: pixels.attr("ID").unwrap_or("").to_string(),
        dimension_order: pixels.attr("DimensionOrder").unwrap_or("").to_string(),
        pixel_type: pixels.attr("Type").unwrap_or("").to_string(),
        size_x: size(pixels, "SizeX")?,
        size_y: size(pixels, "SizeY")?,
        size_z: size(pixels, "SizeZ")?,
        size_c: size(pixels, "SizeC")?,
        size_t: size(pixels, "SizeT")?,
    })
}

/// Read the OME-XML from an OME-TIFF file: the `ImageDescription` of its first image, parsed as OME-XML.
pub fn from_tiff(tiff_bytes: &[u8]) -> Result<Pixels, OmeError> {
    let images =
        tpt_io_tiff::parse(tiff_bytes).map_err(|e| OmeError::Tiff(alloc::format!("{e}")))?;
    let description = images
        .first()
        .and_then(|i| i.description.as_deref())
        .ok_or_else(|| OmeError::Invalid("first IFD has no ImageDescription".to_string()))?;
    parse_xml(description.as_bytes())
}
