//! Tests for the whole-slide tile layout, on a synthetic 10 × 6 image of 4 × 4 tiles (3 across, 2 down).

use tpt_io_dicom::dataset::Dataset;
use tpt_io_dicom::element::DataElement;
use tpt_io_dicom::pixel::PIXEL_DATA;
use tpt_io_dicom::tag::Tag;
use tpt_io_dicom::vr::Vr;
use tpt_io_dicom::wsi::{tile, WsiLayout};

const TOTAL_COLUMNS: Tag = Tag::new(0x0048, 0x0006);
const TOTAL_ROWS: Tag = Tag::new(0x0048, 0x0007);
const ROWS: Tag = Tag::new(0x0028, 0x0010);
const COLUMNS: Tag = Tag::new(0x0028, 0x0011);
const SAMPLES: Tag = Tag::new(0x0028, 0x0002);
const BITS: Tag = Tag::new(0x0028, 0x0100);
const NUMBER_OF_FRAMES: Tag = Tag::new(0x0028, 0x0008);

/// Six frames of 16 bytes; frame `i` is filled with the byte `i`.
fn wsi(frames: u32) -> Dataset {
    let mut ds = Dataset::default();
    ds.upsert(DataElement::ul(TOTAL_COLUMNS, 10));
    ds.upsert(DataElement::ul(TOTAL_ROWS, 6));
    ds.upsert(DataElement::us(ROWS, 4));
    ds.upsert(DataElement::us(COLUMNS, 4));
    ds.upsert(DataElement::us(SAMPLES, 1));
    ds.upsert(DataElement::us(BITS, 8));
    ds.upsert(DataElement::text(NUMBER_OF_FRAMES, Vr::Is, &frames.to_string()));
    let mut pixels = Vec::new();
    for i in 0..6u8 {
        pixels.extend(std::iter::repeat(i).take(16));
    }
    ds.upsert(DataElement::new(PIXEL_DATA, Vr::Ob, pixels));
    ds
}

#[test]
fn layout_counts_tiles_and_frames() {
    let layout = WsiLayout::of(&wsi(6)).unwrap();
    assert_eq!((layout.total_columns, layout.total_rows), (10, 6));
    assert_eq!((layout.tile_columns, layout.tile_rows), (4, 4));
    assert_eq!((layout.tiles_across(), layout.tiles_down()), (3, 2));
    assert_eq!(layout.frames(), 6);
    assert_eq!(layout.focal_planes, 1);
}

#[test]
fn frame_index_is_row_major_and_bounded() {
    let layout = WsiLayout::of(&wsi(6)).unwrap();
    assert_eq!(layout.frame_index(0, 0, 0), Some(0));
    assert_eq!(layout.frame_index(2, 0, 0), Some(2));
    assert_eq!(layout.frame_index(2, 1, 0), Some(5));
    assert_eq!(layout.frame_index(3, 0, 0), None);
    assert_eq!(layout.frame_index(0, 2, 0), None);
    assert_eq!(layout.frame_index(0, 0, 1), None);
}

#[test]
fn tile_returns_the_frame_for_its_position() {
    let ds = wsi(6);
    let layout = WsiLayout::of(&ds).unwrap();
    assert_eq!(tile(&ds, &layout, 1, 0, 0).unwrap(), &[1u8; 16][..]);
    assert_eq!(tile(&ds, &layout, 2, 1, 0).unwrap(), &[5u8; 16][..]);
    assert!(tile(&ds, &layout, 3, 0, 0).is_err());
}

#[test]
fn frame_count_that_disagrees_with_the_layout_is_an_error() {
    let ds = wsi(5);
    let layout = WsiLayout::of(&ds).unwrap();
    assert!(tile(&ds, &layout, 0, 0, 0).is_err());
}

#[test]
fn missing_total_matrix_is_reported() {
    let mut ds = wsi(6);
    assert!(ds.remove(TOTAL_ROWS));
    assert!(WsiLayout::of(&ds).is_err());
}
