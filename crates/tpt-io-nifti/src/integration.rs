//! Feature-gated integration stub for the future `tpt-sci-image` crate.
//!
//! `tpt-sci-image` is not yet published (per `todo.md`). This module provides
//! only the conversion boundary — a stable API plus a local placeholder type — so
//! the `sci-image-integration` feature flag compiles and links today.

#[cfg(feature = "sci-image-integration")]
pub mod sci_image {
    //! Stub conversion into a future `tpt_sci_image` volume type.
    use crate::header::Nifti1Header;
    use crate::model::NiftiFile;

    /// Placeholder for a future `tpt_sci_image::Volume`.
    #[derive(Debug, Clone, PartialEq, Default)]
    pub struct VolumeStub {
        /// Number of voxels.
        pub voxels: usize,
        /// Number of dimensions.
        pub ndim: u8,
        /// Bytes per voxel element.
        pub bytes_per_voxel: usize,
    }

    /// Convert a parsed NIfTI file into a volume placeholder.
    pub fn to_volume(file: &NiftiFile) -> VolumeStub {
        VolumeStub {
            voxels: file.voxel_count(),
            ndim: file.header.ndim,
            bytes_per_voxel: file.header.datatype.size_of(),
        }
    }

    /// Convert just a header into a volume placeholder.
    pub fn header_to_volume(h: &Nifti1Header) -> VolumeStub {
        VolumeStub { voxels: h.voxel_count(), ndim: h.ndim, bytes_per_voxel: h.datatype.size_of() }
    }
}
