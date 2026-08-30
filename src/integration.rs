//! Feature-gated integration stub for the future `tpt-sci-image` crate.
//!
//! `tpt-sci-image` is not yet published (per `todo.md`). This module provides
//! only the conversion boundary — a stable API plus a local placeholder type — so
//! the `sci-image-integration` feature flag compiles and links today.

#[cfg(feature = "sci-image-integration")]
pub mod sci_image {
    //! Stub conversion into a future `tpt_sci_image` image type.
    use crate::model::DicomFile;
    use crate::pixel::PixelDescriptor;

    /// Placeholder for a future `tpt_sci_image::DicomImage`.
    #[derive(Debug, Clone, PartialEq, Default)]
    pub struct DicomImageStub {
        /// Number of rows.
        pub rows: u16,
        /// Number of columns.
        pub columns: u16,
        /// Number of frames.
        pub frames: u32,
        /// Samples per pixel.
        pub samples_per_pixel: u16,
        /// Bits allocated per sample.
        pub bits_allocated: u16,
        /// Photometric interpretation.
        pub photometric: alloc::string::String,
    }

    /// Convert a parsed DICOM file into an image placeholder.
    pub fn to_image_stub(file: &DicomFile) -> Option<DicomImageStub> {
        let desc = crate::pixel::extract_pixel_descriptor(&file.dataset).ok()?;
        Some(DicomImageStub {
            rows: desc.rows,
            columns: desc.columns,
            frames: desc.number_of_frames,
            samples_per_pixel: desc.samples_per_pixel,
            bits_allocated: desc.bits_allocated,
            photometric: desc.photometric_interpretation,
        })
    }

    /// Convert a pixel descriptor into an image placeholder.
    pub fn descriptor_to_image_stub(desc: &PixelDescriptor) -> DicomImageStub {
        DicomImageStub {
            rows: desc.rows,
            columns: desc.columns,
            frames: desc.number_of_frames,
            samples_per_pixel: desc.samples_per_pixel,
            bits_allocated: desc.bits_allocated,
            photometric: desc.photometric_interpretation.clone(),
        }
    }
}
