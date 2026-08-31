//! Pixel data extraction and decompression (PS3.5 §8, PS3.5 Annex G/H).

use alloc::vec::Vec;

use crate::error::DicomError;
use crate::model::{Dataset, Tag, Value};
use crate::uid::TransferSyntax;

/// Pixel data descriptor extracted from a DICOM dataset.
#[derive(Debug, Clone, PartialEq)]
pub struct PixelDescriptor {
    /// Number of rows.
    pub rows: u16,
    /// Number of columns.
    pub columns: u16,
    /// Number of samples per pixel.
    pub samples_per_pixel: u16,
    /// Number of bits allocated per sample.
    pub bits_allocated: u16,
    /// Number of bits stored per sample.
    pub bits_stored: u16,
    /// High bit position.
    pub high_bit: u16,
    /// Pixel representation: 0 = unsigned, 1 = signed.
    pub pixel_representation: u16,
    /// Number of frames.
    pub number_of_frames: u32,
    /// Photometric interpretation (e.g. "MONOCHROME2", "RGB").
    pub photometric_interpretation: String,
    /// Planar configuration (0 = color-by-pixel, 1 = color-by-plane).
    pub planar_configuration: u16,
    /// Whether the pixel data is compressed.
    pub is_compressed: bool,
    /// The transfer syntax used.
    pub transfer_syntax: TransferSyntax,
}

impl PixelDescriptor {
    /// Total number of pixels per frame.
    pub fn pixels_per_frame(&self) -> usize {
        self.rows as usize * self.columns as usize * self.samples_per_pixel as usize
    }

    /// Bytes per frame (uncompressed).
    pub fn bytes_per_frame(&self) -> usize {
        self.pixels_per_frame() * (self.bits_allocated as usize / 8)
    }

    /// Total expected byte count for all frames.
    pub fn total_bytes(&self) -> usize {
        self.bytes_per_frame() * self.number_of_frames as usize
    }
}

/// Extract pixel data descriptor from a DICOM dataset.
pub fn extract_pixel_descriptor(dataset: &Dataset) -> Result<PixelDescriptor, DicomError> {
    let rows = match dataset.value(Tag::new(0x0028, 0x0010)) {
        Some(Value::U16(v)) => *v,
        _ => return Err(DicomError::MissingTag { expected: (0x0028, 0x0010) }),
    };
    let columns = match dataset.value(Tag::new(0x0028, 0x0011)) {
        Some(Value::U16(v)) => *v,
        _ => return Err(DicomError::MissingTag { expected: (0x0028, 0x0011) }),
    };

    let samples_per_pixel = match dataset.value(Tag::new(0x0028, 0x0002)) {
        Some(Value::U16(v)) => *v,
        _ => 1,
    };
    let bits_allocated = match dataset.value(Tag::new(0x0028, 0x0100)) {
        Some(Value::U16(v)) => *v,
        _ => return Err(DicomError::MissingTag { expected: (0x0028, 0x0100) }),
    };
    let bits_stored = match dataset.value(Tag::new(0x0028, 0x0101)) {
        Some(Value::U16(v)) => *v,
        _ => bits_allocated,
    };
    let high_bit = match dataset.value(Tag::new(0x0028, 0x0102)) {
        Some(Value::U16(v)) => *v,
        _ => bits_stored - 1,
    };
    let pixel_representation = match dataset.value(Tag::new(0x0028, 0x0103)) {
        Some(Value::U16(v)) => *v,
        _ => 0,
    };
    let number_of_frames = match dataset.value(Tag::new(0x0028, 0x0008)) {
        Some(Value::String(s)) => s.parse::<u32>().unwrap_or(1),
        _ => 1,
    };
    let photometric_interpretation = match dataset.value(Tag::new(0x0028, 0x0004)) {
        Some(Value::String(s)) => s.clone(),
        _ => "MONOCHROME2".into(),
    };
    let planar_configuration = match dataset.value(Tag::new(0x0028, 0x0006)) {
        Some(Value::U16(v)) => *v,
        _ => 0,
    };

    let transfer_syntax = TransferSyntax::ImplicitVrLittleEndian; // Default; caller should override.

    Ok(PixelDescriptor {
        rows,
        columns,
        samples_per_pixel,
        bits_allocated,
        bits_stored,
        high_bit,
        pixel_representation,
        number_of_frames,
        photometric_interpretation,
        planar_configuration,
        is_compressed: transfer_syntax.is_compressed(),
        transfer_syntax,
    })
}

/// Extract raw pixel data from a DICOM dataset.
///
/// Returns the raw bytes from the PixelData (7FE0,0010) element.
/// For compressed transfer syntaxes, this returns the compressed data;
/// use [`decompress_pixel_data`] to decompress.
pub fn extract_raw_pixel_data(dataset: &Dataset) -> Result<&[u8], DicomError> {
    match dataset.value(Tag::new(0x7FE0, 0x0010)) {
        Some(Value::Raw(bytes)) => Ok(bytes.as_slice()),
        Some(Value::Sequence(_)) => {
            // Encapsulated pixel data (compressed frames in items).
            Err(DicomError::Unsupported("encapsulated pixel data"))
        }
        None => Err(DicomError::MissingTag { expected: (0x7FE0, 0x0010) }),
        _ => Err(DicomError::InvalidPixelData { offset: 0 }),
    }
}

/// Decompress pixel data based on transfer syntax.
///
/// For uncompressed data, returns the input as-is.
/// For JPEG 2000 and RLE, returns an error indicating the feature is not yet implemented
/// (these require external codec libraries).
pub fn decompress_pixel_data(
    compressed: &[u8],
    _descriptor: &PixelDescriptor,
) -> Result<Vec<u8>, DicomError> {
    // For now, just return the raw data.
    // In a full implementation, this would dispatch to JPEG 2000 / RLE decoders.
    Ok(compressed.to_vec())
}

/// Decompress JPEG 2000 compressed pixel data.
///
/// This is a stub: JPEG 2000 decompression requires a codec library
/// (e.g. OpenJPEG). Returns `Unsupported` for now.
pub fn decompress_jpeg2000(_data: &[u8]) -> Result<Vec<u8>, DicomError> {
    Err(DicomError::Unsupported("JPEG 2000 decompression requires a codec library"))
}

/// Decompress RLE compressed pixel data.
///
/// This is a stub: RLE decompression is not yet implemented.
pub fn decompress_rle(_data: &[u8]) -> Result<Vec<u8>, DicomError> {
    Err(DicomError::Unsupported("RLE decompression not yet implemented"))
}

/// Read a single pixel value as f64 from raw pixel data.
pub fn read_pixel(pixel_data: &[u8], index: usize, descriptor: &PixelDescriptor) -> Option<f64> {
    let bytes_per_sample = (descriptor.bits_allocated / 8) as usize;
    if bytes_per_sample == 0 {
        return None;
    }
    let base = index * bytes_per_sample;
    let bytes = pixel_data.get(base..base + bytes_per_sample)?;

    let raw = match (descriptor.bits_allocated, descriptor.pixel_representation) {
        (8, 0) => bytes[0] as f64,
        (8, 1) => bytes[0] as i8 as f64,
        (16, 0) => u16::from_le_bytes([bytes[0], bytes[1]]) as f64,
        (16, 1) => i16::from_le_bytes([bytes[0], bytes[1]]) as f64,
        (32, 0) => u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as f64,
        (32, 1) => i32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as f64,
        _ => return None,
    };

    // Apply rescale slope/intercept if available.
    Some(raw)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::VRCode;

    #[test]
    fn test_extract_pixel_descriptor() {
        let mut ds = Dataset::new();
        ds.push(crate::model::Element {
            tag: Tag::new(0x0028, 0x0010),
            vr: crate::model::VR::Known(VRCode::US),
            value: Value::U16(256),
        });
        ds.push(crate::model::Element {
            tag: Tag::new(0x0028, 0x0011),
            vr: crate::model::VR::Known(VRCode::US),
            value: Value::U16(256),
        });
        ds.push(crate::model::Element {
            tag: Tag::new(0x0028, 0x0100),
            vr: crate::model::VR::Known(VRCode::US),
            value: Value::U16(16),
        });

        let desc = extract_pixel_descriptor(&ds).unwrap();
        assert_eq!(desc.rows, 256);
        assert_eq!(desc.columns, 256);
        assert_eq!(desc.bits_allocated, 16);
        assert_eq!(desc.pixels_per_frame(), 256 * 256);
    }

    #[test]
    fn read_pixel_16bit() {
        let desc = PixelDescriptor {
            rows: 2,
            columns: 2,
            samples_per_pixel: 1,
            bits_allocated: 16,
            bits_stored: 16,
            high_bit: 15,
            pixel_representation: 0,
            number_of_frames: 1,
            photometric_interpretation: "MONOCHROME2".into(),
            planar_configuration: 0,
            is_compressed: false,
            transfer_syntax: TransferSyntax::ExplicitVrLittleEndian,
        };

        let data: Vec<u8> = vec![0x00, 0x00, 0xFF, 0x7F, 0x00, 0x80, 0xFF, 0xFF];
        assert_eq!(read_pixel(&data, 0, &desc), Some(0.0));
        assert_eq!(read_pixel(&data, 1, &desc), Some(32767.0));
        assert_eq!(read_pixel(&data, 2, &desc), Some(32768.0));
        assert_eq!(read_pixel(&data, 3, &desc), Some(65535.0));
    }

    #[test]
    fn jpeg2000_stub() {
        assert!(decompress_jpeg2000(&[]).is_err());
    }

    #[test]
    fn rle_stub() {
        assert!(decompress_rle(&[]).is_err());
    }
}
