//! Whole-slide imaging (WSI) layout: tiled frames over a total pixel matrix.
//!
//! A whole-slide image (the VL Whole Slide Microscopy storage class, PS3.3 A.32.8) stores its pixels as frames,
//! one per tile. The tiles cover a `Total Pixel Matrix Columns` by `Total Pixel Matrix Rows` image, each tile
//! `Columns` by `Rows` pixels. Frames run row by row within each focal plane, and the focal planes follow one
//! another. This module reads that layout, maps tile positions to frame indices, and returns the native bytes of
//! a tile. Encapsulated (compressed) WSI frames are not decoded here; use [`crate::pixel::encapsulated_fragments`]
//! to reach their fragments.
//!
//! Spec access: DICOM PS3.3 is published by NEMA under a free licence. No specification text is copied into this
//! crate.

use alloc::string::ToString;

use crate::dataset::Dataset;
use crate::error::{DicomError, Result};
use crate::pixel::{frame_geometry, native_pixel_data};
use crate::tag::Tag;

const TOTAL_COLUMNS: Tag = Tag::new(0x0048, 0x0006);
const TOTAL_ROWS: Tag = Tag::new(0x0048, 0x0007);
const FOCAL_PLANES: Tag = Tag::new(0x0048, 0x0001);

/// The tile layout of a whole-slide image.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WsiLayout {
    /// Pixels across the whole image (Total Pixel Matrix Columns).
    pub total_columns: u32,
    /// Pixels down the whole image (Total Pixel Matrix Rows).
    pub total_rows: u32,
    /// Pixels per tile across (Columns).
    pub tile_columns: u32,
    /// Pixels per tile down (Rows).
    pub tile_rows: u32,
    /// The number of focal planes (1 when the element is absent).
    pub focal_planes: u32,
}

impl WsiLayout {
    /// Read the layout from a dataset: the total pixel matrix, the focal planes, and the tile size from the image
    /// pixel module.
    pub fn of(ds: &Dataset) -> Result<WsiLayout> {
        let geometry = frame_geometry(ds)?;
        let total = |tag: Tag| {
            ds.u32_at(tag)
                .ok_or_else(|| DicomError::MissingRequiredElement { tag: tag.to_string() })
        };
        let focal_planes = match ds.is_at(FOCAL_PLANES) {
            Some(n) if n > 0 => n as u32,
            _ => 1,
        };
        if geometry.rows == 0 || geometry.columns == 0 {
            return Err(DicomError::invalid_value(0, "tile rows and columns must be non-zero"));
        }
        Ok(WsiLayout {
            total_columns: total(TOTAL_COLUMNS)?,
            total_rows: total(TOTAL_ROWS)?,
            tile_columns: geometry.columns as u32,
            tile_rows: geometry.rows as u32,
            focal_planes,
        })
    }

    /// Tiles across the image: the total columns divided by the tile width, rounded up.
    pub fn tiles_across(&self) -> u32 {
        self.total_columns.div_ceil(self.tile_columns)
    }

    /// Tiles down the image: the total rows divided by the tile height, rounded up.
    pub fn tiles_down(&self) -> u32 {
        self.total_rows.div_ceil(self.tile_rows)
    }

    /// The number of frames the layout needs: one per tile per focal plane.
    pub fn frames(&self) -> u32 {
        self.tiles_across() * self.tiles_down() * self.focal_planes
    }

    /// The frame index of the tile at `(column, row)` on `plane`, or `None` when that position is outside the image.
    pub fn frame_index(&self, column: u32, row: u32, plane: u32) -> Option<u32> {
        if column >= self.tiles_across() || row >= self.tiles_down() || plane >= self.focal_planes {
            return None;
        }
        Some(plane * self.tiles_across() * self.tiles_down() + row * self.tiles_across() + column)
    }
}

/// The native bytes of one tile of a whole-slide image with uncompressed pixel data.
///
/// Fails when the position is outside the image, when `NumberOfFrames` does not match the layout, or when the pixel
/// data is shorter than the layout needs.
pub fn tile<'a>(
    ds: &'a Dataset,
    layout: &WsiLayout,
    column: u32,
    row: u32,
    plane: u32,
) -> Result<&'a [u8]> {
    let (bytes, geometry) = native_pixel_data(ds)?;
    if geometry.number_of_frames != layout.frames() {
        return Err(DicomError::invalid_value(0, "NumberOfFrames does not match the tile layout"));
    }
    let index = layout
        .frame_index(column, row, plane)
        .ok_or_else(|| DicomError::invalid_value(0, "tile position is outside the image"))?;
    let size = geometry.frame_bytes();
    let start = index as usize * size;
    bytes.get(start..start + size).ok_or_else(|| {
        DicomError::invalid_value(start, "pixel data is shorter than the tile layout needs")
    })
}
