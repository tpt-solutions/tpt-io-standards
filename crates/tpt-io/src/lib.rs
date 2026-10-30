//! Umbrella crate for the `tpt-io` multi-format parser workspace.
//!
//! Each format parser is an independent crate re-exported here behind an opt-in
//! Cargo feature (e.g. `features = ["dicom", "ifc"]`). Compiling `tpt-io` with
//! **no** features is a true zero-cost crate that only re-exports
//! `tpt-io-core`. The feature tree is flat (no transitive feature deps).
//!
//! See `spec.txt §3` and the workspace `README.md` for the design and usage.

#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]

#[cfg(any(feature = "alloc", feature = "std"))]
extern crate alloc;

pub mod detect;

pub use tpt_io_core::{
    Encoding, FormatExtension, NoProgress, Progress, ProgressFn, Reader, Schema,
    ValidationSettings, Validator,
};
#[cfg(any(feature = "alloc", feature = "std"))]
pub use tpt_io_core::{ParseError, Writer};

/// Unified error wrapper: wraps each enabled domain parser's error with the name
/// of the format that produced it. Only the variants for enabled formats are
/// present, versus a `Core` variant for errors from `tpt-io-core`.
#[cfg(any(feature = "alloc", feature = "std"))]
#[derive(Debug)]
#[non_exhaustive]
pub enum IoError {
    /// An error originating from the shared core crate.
    Core(ParseError),
    #[cfg(feature = "dicom")]
    Dicom(tpt_io_dicom::DicomError),
    #[cfg(feature = "fhir")]
    Fhir(tpt_io_fhir::FhirError),
    #[cfg(feature = "nifti")]
    Nifti(tpt_io_nifti::NiftiError),
    #[cfg(feature = "ifc")]
    Ifc(tpt_io_ifc::IfcError),
    #[cfg(feature = "citygml")]
    CityGml(tpt_io_citygml::CityGmlError),
    #[cfg(feature = "spice")]
    Spice(tpt_io_spice::SpiceError),
    #[cfg(feature = "gerber")]
    Gerber(tpt_io_gerber::GerberError),
    #[cfg(feature = "edif")]
    Edif(tpt_io_edif::EdifError),
    #[cfg(feature = "geo")]
    Geo(tpt_io_geo::GeoError),
    #[cfg(feature = "netcdf")]
    NetCdf(tpt_io_netcdf::NetCdfError),
    #[cfg(feature = "welllog")]
    WellLog(tpt_io_welllog::WellLogError),
    #[cfg(feature = "capeopen")]
    CapeOpen(tpt_io_capeopen::CapeOpenError),
    #[cfg(feature = "sbml")]
    Sbml(tpt_io_sbml::SbmlError),
    #[cfg(feature = "hdf5")]
    Hdf5(tpt_io_hdf5::Hdf5Error),
    #[cfg(feature = "kicad")]
    Kicad(tpt_io_kicad::KicadError),
}

#[cfg(any(feature = "alloc", feature = "std"))]
impl core::fmt::Display for IoError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            IoError::Core(e) => write!(f, "core: {e}"),
            #[cfg(feature = "dicom")]
            IoError::Dicom(e) => write!(f, "DICOM: {e}"),
            #[cfg(feature = "fhir")]
            IoError::Fhir(e) => write!(f, "FHIR: {e}"),
            #[cfg(feature = "nifti")]
            IoError::Nifti(e) => write!(f, "NIfTI: {e}"),
            #[cfg(feature = "ifc")]
            IoError::Ifc(e) => write!(f, "IFC: {e}"),
            #[cfg(feature = "citygml")]
            IoError::CityGml(e) => write!(f, "CityGML: {e}"),
            #[cfg(feature = "spice")]
            IoError::Spice(e) => write!(f, "SPICE: {e}"),
            #[cfg(feature = "gerber")]
            IoError::Gerber(e) => write!(f, "Gerber: {e}"),
            #[cfg(feature = "edif")]
            IoError::Edif(e) => write!(f, "EDIF: {e}"),
            #[cfg(feature = "geo")]
            IoError::Geo(e) => write!(f, "Geo: {e}"),
            #[cfg(feature = "netcdf")]
            IoError::NetCdf(e) => write!(f, "NetCDF: {e}"),
            #[cfg(feature = "welllog")]
            IoError::WellLog(e) => write!(f, "WellLog: {e}"),
            #[cfg(feature = "capeopen")]
            IoError::CapeOpen(e) => write!(f, "CAPE-OPEN: {e}"),
            #[cfg(feature = "sbml")]
            IoError::Sbml(e) => write!(f, "SBML: {e}"),
            #[cfg(feature = "hdf5")]
            IoError::Hdf5(e) => write!(f, "HDF5: {e}"),
            #[cfg(feature = "kicad")]
            IoError::Kicad(e) => write!(f, "KiCad: {e}"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for IoError {}

// Re-export each domain crate as a module behind its feature.
#[cfg(feature = "capeopen")]
pub use tpt_io_capeopen as capeopen;
#[cfg(feature = "citygml")]
pub use tpt_io_citygml as citygml;
#[cfg(feature = "dicom")]
pub use tpt_io_dicom as dicom;
#[cfg(feature = "edif")]
pub use tpt_io_edif as edif;
#[cfg(feature = "fhir")]
pub use tpt_io_fhir as fhir;
#[cfg(feature = "geo")]
pub use tpt_io_geo as geo;
#[cfg(feature = "gerber")]
pub use tpt_io_gerber as gerber;
#[cfg(feature = "hdf5")]
pub use tpt_io_hdf5 as hdf5;
#[cfg(feature = "ifc")]
pub use tpt_io_ifc as ifc;
#[cfg(feature = "kicad")]
pub use tpt_io_kicad as kicad;
#[cfg(feature = "netcdf")]
pub use tpt_io_netcdf as netcdf;
#[cfg(feature = "nifti")]
pub use tpt_io_nifti as nifti;
#[cfg(feature = "sbml")]
pub use tpt_io_sbml as sbml;
#[cfg(feature = "spice")]
pub use tpt_io_spice as spice;
#[cfg(feature = "welllog")]
pub use tpt_io_welllog as welllog;
