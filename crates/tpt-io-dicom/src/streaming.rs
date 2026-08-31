//! Streaming pixel data support for large volumetric datasets (CT/MRI).
//!
//! For large multi-frame datasets, this module provides an iterator-based
//! interface that reads and decompresses frames one at a time, avoiding the
//! need to load the entire dataset into memory.

use alloc::vec::Vec;

use crate::error::DicomError;
use crate::model::Dataset;
use crate::pixel::{decompress_pixel_data, extract_pixel_descriptor, PixelDescriptor};

/// A streaming pixel data reader for large volumetric datasets.
#[derive(Debug)]
pub struct PixelStream<'a> {
    /// The dataset containing pixel data.
    dataset: &'a Dataset,
    /// The pixel descriptor.
    descriptor: PixelDescriptor,
    /// Raw pixel data bytes.
    raw_data: &'a [u8],
    /// Current frame index.
    current_frame: u32,
}

impl<'a> PixelStream<'a> {
    /// Create a new pixel stream from a dataset.
    pub fn new(dataset: &'a Dataset) -> Result<Self, DicomError> {
        let mut descriptor = extract_pixel_descriptor(dataset)?;
        let raw_data = crate::pixel::extract_raw_pixel_data(dataset)?;

        // Determine transfer syntax from dataset if available.
        // For now, we use the default from descriptor.
        Ok(Self { dataset, descriptor, raw_data, current_frame: 0 })
    }

    /// Get the pixel descriptor.
    pub fn descriptor(&self) -> &PixelDescriptor {
        &self.descriptor
    }

    /// Total number of frames.
    pub fn frame_count(&self) -> u32 {
        self.descriptor.number_of_frames
    }

    /// Read a single frame by index.
    pub fn read_frame(&self, frame_index: u32) -> Result<Vec<u8>, DicomError> {
        if frame_index >= self.descriptor.number_of_frames {
            return Err(DicomError::InvalidPixelData { offset: frame_index as usize });
        }
        let frame_size = self.descriptor.bytes_per_frame();
        let offset = frame_index as usize * frame_size;
        let end = offset + frame_size;

        if end > self.raw_data.len() {
            return Err(DicomError::InvalidPixelData { offset });
        }

        let frame_data = &self.raw_data[offset..end];

        if self.descriptor.is_compressed {
            decompress_pixel_data(frame_data, &self.descriptor)
        } else {
            Ok(frame_data.to_vec())
        }
    }

    /// Iterate over all frames.
    pub fn frames(&self) -> FrameIter<'_> {
        FrameIter { stream: self, current: 0 }
    }

    /// Iterate over a range of frames.
    pub fn frame_range(&self, start: u32, end: u32) -> FrameIter<'_> {
        FrameIter { stream: self, current: start.min(self.descriptor.number_of_frames) }
    }
}

/// Iterator over frames in a pixel stream.
#[derive(Debug)]
pub struct FrameIter<'a> {
    stream: &'a PixelStream<'a>,
    current: u32,
}

impl<'a> Iterator for FrameIter<'a> {
    type Item = Result<Vec<u8>, DicomError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current >= self.stream.descriptor.number_of_frames {
            return None;
        }
        let result = self.stream.read_frame(self.current);
        self.current += 1;
        Some(result)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = (self.stream.descriptor.number_of_frames - self.current) as usize;
        (remaining, Some(remaining))
    }
}

impl ExactSizeIterator for FrameIter<'_> {}

/// Chunked pixel data reader for very large datasets.
///
/// Reads pixel data in fixed-size chunks rather than frame-by-frame,
/// useful for streaming processing of large volumes.
#[derive(Debug)]
pub struct ChunkedPixelReader<'a> {
    /// Raw pixel data.
    raw_data: &'a [u8],
    /// Current read position.
    position: usize,
    /// Chunk size in bytes.
    chunk_size: usize,
}

impl<'a> ChunkedPixelReader<'a> {
    /// Create a new chunked reader.
    pub fn new(raw_data: &'a [u8], chunk_size: usize) -> Self {
        Self { raw_data, position: 0, chunk_size: chunk_size.max(1) }
    }

    /// Read the next chunk of pixel data.
    pub fn read_chunk(&mut self) -> Option<&'a [u8]> {
        if self.position >= self.raw_data.len() {
            return None;
        }
        let end = (self.position + self.chunk_size).min(self.raw_data.len());
        let chunk = &self.raw_data[self.position..end];
        self.position = end;
        Some(chunk)
    }

    /// Remaining bytes.
    pub fn remaining(&self) -> usize {
        self.raw_data.len() - self.position
    }

    /// Total length.
    pub fn len(&self) -> usize {
        self.raw_data.len()
    }

    /// Whether all data has been read.
    pub fn is_empty(&self) -> bool {
        self.position >= self.raw_data.len()
    }
}

impl<'a> Iterator for ChunkedPixelReader<'a> {
    type Item = &'a [u8];

    fn next(&mut self) -> Option<Self::Item> {
        self.read_chunk()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Element, Tag, Value, VR};
    use crate::tag::VRCode;

    fn make_test_dataset_with_pixels() -> Dataset {
        let mut ds = Dataset::new();
        ds.push(Element {
            tag: Tag::new(0x0028, 0x0010),
            vr: VR::Known(VRCode::US),
            value: Value::U16(4),
        });
        ds.push(Element {
            tag: Tag::new(0x0028, 0x0011),
            vr: VR::Known(VRCode::US),
            value: Value::U16(4),
        });
        ds.push(Element {
            tag: Tag::new(0x0028, 0x0100),
            vr: VR::Known(VRCode::US),
            value: Value::U16(16),
        });
        ds.push(Element {
            tag: Tag::new(0x7FE0, 0x0010),
            vr: VR::Known(VRCode::OW),
            value: Value::Raw(vec![0u8; 4 * 4 * 2]), // 4x4 16-bit image
        });
        ds
    }

    #[test]
    fn pixel_stream_frame_count() {
        let ds = make_test_dataset_with_pixels();
        let stream = PixelStream::new(&ds).unwrap();
        assert_eq!(stream.frame_count(), 1);
    }

    #[test]
    fn pixel_stream_read_frame() {
        let mut ds = Dataset::new();
        ds.push(Element {
            tag: Tag::new(0x0028, 0x0010),
            vr: VR::Known(VRCode::US),
            value: Value::U16(2),
        });
        ds.push(Element {
            tag: Tag::new(0x0028, 0x0011),
            vr: VR::Known(VRCode::US),
            value: Value::U16(2),
        });
        ds.push(Element {
            tag: Tag::new(0x0028, 0x0100),
            vr: VR::Known(VRCode::US),
            value: Value::U16(16),
        });
        // 2x2 16-bit image = 8 bytes.
        ds.push(Element {
            tag: Tag::new(0x7FE0, 0x0010),
            vr: VR::Known(VRCode::OW),
            value: Value::Raw(vec![0x01, 0x00, 0x02, 0x00, 0x03, 0x00, 0x04, 0x00]),
        });

        let stream = PixelStream::new(&ds).unwrap();
        let frame = stream.read_frame(0).unwrap();
        assert_eq!(frame.len(), 8);
    }

    #[test]
    fn chunked_reader() {
        let data = vec![1u8, 2, 3, 4, 5, 6, 7, 8, 9, 10];
        let mut reader = ChunkedPixelReader::new(&data, 3);
        assert_eq!(reader.read_chunk(), Some(&[1, 2, 3][..]));
        assert_eq!(reader.read_chunk(), Some(&[4, 5, 6][..]));
        assert_eq!(reader.read_chunk(), Some(&[7, 8, 9][..]));
        assert_eq!(reader.read_chunk(), Some(&[10][..]));
        assert_eq!(reader.read_chunk(), None);
    }
}
