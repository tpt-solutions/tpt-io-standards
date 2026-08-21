//! Integration bridge with the future `tpt-sci-image` crate
//! (feature: `sci-image-integration`, off by default).
//!
//! `tpt-sci-image` is not yet published, so this module only exposes a
//! neutral, dependency-free description of a decoded frame. Once that crate
//! exists, its image types can be produced from [`DecodedFrame`] without
//! touching the parser.

use alloc::vec::Vec;

/// A decoded, uncompressed frame plus the metadata needed to interpret it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodedFrame {
    /// Raw samples in the file's native layout (little endian, interleaved).
    pub samples: Vec<u8>,
    /// Rows.
    pub rows: u16,
    /// Columns.
    pub columns: u16,
    /// Samples per pixel.
    pub samples_per_pixel: u16,
    /// Bits allocated.
    pub bits_allocated: u16,
    /// 0 = unsigned, 1 = two's complement signed (PixelRepresentation).
    pub pixel_representation: u16,
    /// Photometric interpretation string (e.g. "MONOCHROME2", "RGB").
    pub photometric_interpretation: alloc::string::String,
}

/// Decode all frames of a dataset into [`DecodedFrame`] values.
pub fn decode_frames(
    ds: &crate::dataset::Dataset,
    ts: crate::uid::TransferSyntax,
) -> crate::error::Result<Vec<DecodedFrame>> {
    let frames = crate::pixel::decode_pixel_data(ds, ts)?;
    let get16 = |t: crate::tag::Tag| ds.u16_at(t).unwrap_or(0);
    Ok(frames
        .into_iter()
        .map(|samples| DecodedFrame {
            samples,
            rows: get16(crate::tag::Tag::new(0x0028, 0x0010)),
            columns: get16(crate::tag::Tag::new(0x0028, 0x0011)),
            samples_per_pixel: get16(crate::tag::Tag::new(0x0028, 0x0002)),
            bits_allocated: get16(crate::tag::Tag::new(0x0028, 0x0100)),
            pixel_representation: get16(crate::tag::Tag::new(0x0028, 0x0103)),
            photometric_interpretation: ds
                .string(crate::tag::Tag::new(0x0028, 0x0004))
                .unwrap_or_default(),
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::decode_frames;
    use crate::dataset::{Dataset, Encoding};
    use crate::element::DataElement;
    use crate::tag::Tag;
    use crate::vr::Vr;

    #[test]
    fn decoded_frame_metadata() {
        let mut ds = Dataset {
            elements: Vec::new(),
            encoding: Encoding { big_endian: false, explicit_vr: true },
        };
        ds.upsert(DataElement::us(Tag::new(0x0028, 0x0010), 1));
        ds.upsert(DataElement::us(Tag::new(0x0028, 0x0011), 1));
        ds.upsert(DataElement::us(Tag::new(0x0028, 0x0002), 1));
        ds.upsert(DataElement::us(Tag::new(0x0028, 0x0100), 16));
        ds.upsert(DataElement::us(Tag::new(0x0028, 0x0103), 1));
        ds.upsert(DataElement::text(Tag::new(0x0028, 0x0004), Vr::Cs, "MONOCHROME2 "));
        ds.upsert(DataElement::new(Tag::new(0x7FE0, 0x0010), Vr::Ob, vec![1, 2]));
        let frames =
            decode_frames(&ds, crate::uid::TransferSyntax::ExplicitVrLittleEndian).unwrap();
        assert_eq!(frames.len(), 1);
        let f = &frames[0];
        assert_eq!(f.samples, vec![1, 2]);
        assert_eq!(f.bits_allocated, 16);
        assert_eq!(f.pixel_representation, 1);
        assert_eq!(f.photometric_interpretation, "MONOCHROME2");
    }
}
