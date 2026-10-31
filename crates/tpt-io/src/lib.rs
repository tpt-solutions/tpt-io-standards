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
    #[cfg(feature = "mdf")]
    Mdf(tpt_io_mdf::MdfError),
    #[cfg(feature = "grib")]
    Grib(tpt_io_grib::GribError),
    #[cfg(feature = "naif")]
    Naif(tpt_io_naif::NaifError),
    #[cfg(feature = "omex")]
    Omex(tpt_io_omex::OmexError),
    #[cfg(feature = "bcf")]
    Bcf(tpt_io_bcf::BcfError),
    #[cfg(feature = "threemf")]
    ThreeMf(tpt_io_threemf::ThreeMfError),
    #[cfg(feature = "mesh")]
    Mesh(tpt_io_mesh::MeshError),
    #[cfg(feature = "gnss")]
    Gnss(tpt_io_gnss::GnssError),
    #[cfg(feature = "hecras")]
    HecRas(tpt_io_hecras::HecRasError),
    #[cfg(feature = "ome")]
    Ome(tpt_io_ome::OmeError),
    #[cfg(feature = "tiff")]
    Tiff(tpt_io_tiff::TiffError),
    #[cfg(feature = "pointcloud")]
    PointCloud(tpt_io_pointcloud::LasError),
    #[cfg(feature = "segy")]
    Segy(tpt_io_segy::SegyError),
    #[cfg(feature = "gds")]
    Gds(tpt_io_gds::GdsError),
    #[cfg(feature = "edf")]
    Edf(tpt_io_edf::EdfError),
    #[cfg(feature = "openehr")]
    OpenEhr(tpt_io_openehr::OpenEhrError),
    #[cfg(feature = "nrrd")]
    Nrrd(tpt_io_nrrd::NrrdError),
    #[cfg(feature = "cellml")]
    CellMl(tpt_io_cellml::CellMlError),
    #[cfg(feature = "dexpi")]
    Dexpi(tpt_io_dexpi::DexpiError),
    #[cfg(feature = "automationml")]
    AutomationMl(tpt_io_automationml::AutomationMlError),
    #[cfg(feature = "witsml")]
    Witsml(tpt_io_witsml::WitsmlError),
    #[cfg(feature = "ipc2581")]
    Ipc2581(tpt_io_ipc2581::Ipc2581Error),
    #[cfg(feature = "cda")]
    Cda(tpt_io_cda::CdaError),
    #[cfg(feature = "gbxml")]
    GbXml(tpt_io_gbxml::GbXmlError),
    #[cfg(feature = "landxml")]
    LandXml(tpt_io_landxml::LandXmlError),
    #[cfg(feature = "mzml")]
    MzMl(tpt_io_mzml::MzmlError),
    #[cfg(feature = "jcamp")]
    Jcamp(tpt_io_jcamp::JcampError),
    #[cfg(feature = "chem")]
    Chem(tpt_io_chem::ChemError),
    #[cfg(feature = "pdb")]
    Pdb(tpt_io_pdb::PdbError),
    #[cfg(feature = "dxf")]
    Dxf(tpt_io_dxf::DxfError),
    #[cfg(feature = "hl7v2")]
    Hl7v2(tpt_io_hl7v2::Hl7Error),
    #[cfg(feature = "verilog")]
    Verilog(tpt_io_verilog::VerilogError),
    #[cfg(feature = "touchstone")]
    Touchstone(tpt_io_touchstone::TouchstoneError),
    #[cfg(feature = "chemfile")]
    ChemFile(tpt_io_chemfile::ChemError),
    #[cfg(feature = "cif")]
    Cif(tpt_io_cif::CifError),
    #[cfg(feature = "asterix")]
    Asterix(tpt_io_asterix::AsterixError),
    #[cfg(feature = "aixm")]
    Aixm(tpt_io_aixm::AixmError),
    #[cfg(feature = "opcua")]
    OpcUa(tpt_io_opcua::OpcUaError),
    #[cfg(feature = "aas")]
    Aas(tpt_io_aas::AasError),
    #[cfg(feature = "plcopen")]
    PlcOpen(tpt_io_plcopen::PlcOpenError),
    #[cfg(feature = "mtconnect")]
    MtConnect(tpt_io_mtconnect::MtConnectError),
    #[cfg(feature = "waterml")]
    WaterMl(tpt_io_waterml::WaterMlError),
    #[cfg(feature = "epanet")]
    Epanet(tpt_io_epanet::EpanetError),
    #[cfg(feature = "gcode")]
    GCode(tpt_io_gcode::GcodeError),
    #[cfg(feature = "fits")]
    Fits(tpt_io_fits::FitsError),
    #[cfg(feature = "ccsds")]
    Ccsds(tpt_io_ccsds::CcsdsError),
    #[cfg(feature = "tle")]
    Tle(tpt_io_tle::TleError),
    #[cfg(feature = "arxml")]
    Arxml(tpt_io_arxml::ArxmlError),
    #[cfg(feature = "openscenario")]
    OpenScenario(tpt_io_openscenario::OpenScenarioError),
    #[cfg(feature = "opendrive")]
    OpenDrive(tpt_io_opendrive::OpenDriveError),
    #[cfg(feature = "urdf")]
    Urdf(tpt_io_urdf::UrdfError),
    #[cfg(feature = "rosbag")]
    Rosbag(tpt_io_rosbag::RosbagError),
    #[cfg(feature = "mcap")]
    Mcap(tpt_io_mcap::McapError),
    #[cfg(feature = "gridlabd")]
    GridLabd(tpt_io_gridlabd::GlmError),
    #[cfg(feature = "opendss")]
    OpenDss(tpt_io_opendss::OpenDssError),
    #[cfg(feature = "matpower")]
    Matpower(tpt_io_matpower::MatpowerError),
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
            #[cfg(feature = "mdf")]
            IoError::Mdf(e) => write!(f, "ASAM MDF 4: {e}"),
            #[cfg(feature = "grib")]
            IoError::Grib(e) => write!(f, "GRIB2: {e}"),
            #[cfg(feature = "naif")]
            IoError::Naif(e) => write!(f, "NAIF DAF/SPK: {e}"),
            #[cfg(feature = "omex")]
            IoError::Omex(e) => write!(f, "COMBINE OMEX / SED-ML: {e}"),
            #[cfg(feature = "bcf")]
            IoError::Bcf(e) => write!(f, "BCF 2.1: {e}"),
            #[cfg(feature = "threemf")]
            IoError::ThreeMf(e) => write!(f, "3MF: {e}"),
            #[cfg(feature = "mesh")]
            IoError::Mesh(e) => write!(f, "STL / OBJ / PLY: {e}"),
            #[cfg(feature = "gnss")]
            IoError::Gnss(e) => write!(f, "RINEX 3 / SP3: {e}"),
            #[cfg(feature = "hecras")]
            IoError::HecRas(e) => write!(f, "HEC-RAS geometry: {e}"),
            #[cfg(feature = "ome")]
            IoError::Ome(e) => write!(f, "OME (TIFF/Zarr): {e}"),
            #[cfg(feature = "tiff")]
            IoError::Tiff(e) => write!(f, "TIFF: {e}"),
            #[cfg(feature = "pointcloud")]
            IoError::PointCloud(e) => write!(f, "LAS point cloud: {e}"),
            #[cfg(feature = "segy")]
            IoError::Segy(e) => write!(f, "SEG-Y: {e}"),
            #[cfg(feature = "gds")]
            IoError::Gds(e) => write!(f, "GDSII: {e}"),
            #[cfg(feature = "edf")]
            IoError::Edf(e) => write!(f, "EDF / BDF: {e}"),
            #[cfg(feature = "openehr")]
            IoError::OpenEhr(e) => write!(f, "openEHR composition: {e}"),
            #[cfg(feature = "nrrd")]
            IoError::Nrrd(e) => write!(f, "NRRD: {e}"),
            #[cfg(feature = "cellml")]
            IoError::CellMl(e) => write!(f, "CellML 1.x: {e}"),
            #[cfg(feature = "dexpi")]
            IoError::Dexpi(e) => write!(f, "DEXPI P&ID: {e}"),
            #[cfg(feature = "automationml")]
            IoError::AutomationMl(e) => write!(f, "AutomationML (CAEX): {e}"),
            #[cfg(feature = "witsml")]
            IoError::Witsml(e) => write!(f, "WITSML trajectories: {e}"),
            #[cfg(feature = "ipc2581")]
            IoError::Ipc2581(e) => write!(f, "IPC-2581: {e}"),
            #[cfg(feature = "cda")]
            IoError::Cda(e) => write!(f, "HL7 CDA / C-CDA: {e}"),
            #[cfg(feature = "gbxml")]
            IoError::GbXml(e) => write!(f, "gbXML: {e}"),
            #[cfg(feature = "landxml")]
            IoError::LandXml(e) => write!(f, "LandXML: {e}"),
            #[cfg(feature = "mzml")]
            IoError::MzMl(e) => write!(f, "mzML: {e}"),
            #[cfg(feature = "jcamp")]
            IoError::Jcamp(e) => write!(f, "JCAMP-DX: {e}"),
            #[cfg(feature = "chem")]
            IoError::Chem(e) => write!(f, "MOL / SDF: {e}"),
            #[cfg(feature = "pdb")]
            IoError::Pdb(e) => write!(f, "PDB: {e}"),
            #[cfg(feature = "dxf")]
            IoError::Dxf(e) => write!(f, "DXF (ASCII): {e}"),
            #[cfg(feature = "hl7v2")]
            IoError::Hl7v2(e) => write!(f, "HL7 v2: {e}"),
            #[cfg(feature = "verilog")]
            IoError::Verilog(e) => write!(f, "Verilog netlist: {e}"),
            #[cfg(feature = "touchstone")]
            IoError::Touchstone(e) => write!(f, "Touchstone S-parameters: {e}"),
            #[cfg(feature = "chemfile")]
            IoError::ChemFile(e) => write!(f, "XYZ/cube/POSCAR/LAMMPS: {e}"),
            #[cfg(feature = "cif")]
            IoError::Cif(e) => write!(f, "CIF / mmCIF: {e}"),
            #[cfg(feature = "asterix")]
            IoError::Asterix(e) => write!(f, "ASTERIX: {e}"),
            #[cfg(feature = "aixm")]
            IoError::Aixm(e) => write!(f, "AIXM 5.1: {e}"),
            #[cfg(feature = "opcua")]
            IoError::OpcUa(e) => write!(f, "OPC UA NodeSet: {e}"),
            #[cfg(feature = "aas")]
            IoError::Aas(e) => write!(f, "AAS JSON: {e}"),
            #[cfg(feature = "plcopen")]
            IoError::PlcOpen(e) => write!(f, "PLCopen XML: {e}"),
            #[cfg(feature = "mtconnect")]
            IoError::MtConnect(e) => write!(f, "MTConnect: {e}"),
            #[cfg(feature = "waterml")]
            IoError::WaterMl(e) => write!(f, "WaterML 2.0: {e}"),
            #[cfg(feature = "epanet")]
            IoError::Epanet(e) => write!(f, "EPANET/SWMM: {e}"),
            #[cfg(feature = "gcode")]
            IoError::GCode(e) => write!(f, "G-code: {e}"),
            #[cfg(feature = "fits")]
            IoError::Fits(e) => write!(f, "FITS: {e}"),
            #[cfg(feature = "ccsds")]
            IoError::Ccsds(e) => write!(f, "CCSDS OEM/OPM/TDM: {e}"),
            #[cfg(feature = "tle")]
            IoError::Tle(e) => write!(f, "TLE: {e}"),
            #[cfg(feature = "arxml")]
            IoError::Arxml(e) => write!(f, "AUTOSAR ARXML: {e}"),
            #[cfg(feature = "openscenario")]
            IoError::OpenScenario(e) => write!(f, "OpenSCENARIO: {e}"),
            #[cfg(feature = "opendrive")]
            IoError::OpenDrive(e) => write!(f, "OpenDRIVE: {e}"),
            #[cfg(feature = "urdf")]
            IoError::Urdf(e) => write!(f, "URDF/SDF: {e}"),
            #[cfg(feature = "rosbag")]
            IoError::Rosbag(e) => write!(f, "ROS bag: {e}"),
            #[cfg(feature = "mcap")]
            IoError::Mcap(e) => write!(f, "MCAP: {e}"),
            #[cfg(feature = "gridlabd")]
            IoError::GridLabd(e) => write!(f, "GridLAB-D: {e}"),
            #[cfg(feature = "opendss")]
            IoError::OpenDss(e) => write!(f, "OpenDSS: {e}"),
            #[cfg(feature = "matpower")]
            IoError::Matpower(e) => write!(f, "MATPOWER: {e}"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for IoError {}

// Re-export each domain crate as a module behind its feature.
#[cfg(feature = "aas")]
pub use tpt_io_aas as aas;
#[cfg(feature = "aixm")]
pub use tpt_io_aixm as aixm;
#[cfg(feature = "arxml")]
pub use tpt_io_arxml as arxml;
#[cfg(feature = "asterix")]
pub use tpt_io_asterix as asterix;
#[cfg(feature = "automationml")]
pub use tpt_io_automationml as automationml;
#[cfg(feature = "bcf")]
pub use tpt_io_bcf as bcf;
#[cfg(feature = "capeopen")]
pub use tpt_io_capeopen as capeopen;
#[cfg(feature = "ccsds")]
pub use tpt_io_ccsds as ccsds;
#[cfg(feature = "cda")]
pub use tpt_io_cda as cda;
#[cfg(feature = "cellml")]
pub use tpt_io_cellml as cellml;
#[cfg(feature = "chem")]
pub use tpt_io_chem as chem;
#[cfg(feature = "chemfile")]
pub use tpt_io_chemfile as chemfile;
#[cfg(feature = "cif")]
pub use tpt_io_cif as cif;
#[cfg(feature = "citygml")]
pub use tpt_io_citygml as citygml;
#[cfg(feature = "dexpi")]
pub use tpt_io_dexpi as dexpi;
#[cfg(feature = "dicom")]
pub use tpt_io_dicom as dicom;
#[cfg(feature = "dxf")]
pub use tpt_io_dxf as dxf;
#[cfg(feature = "edf")]
pub use tpt_io_edf as edf;
#[cfg(feature = "edif")]
pub use tpt_io_edif as edif;
#[cfg(feature = "epanet")]
pub use tpt_io_epanet as epanet;
#[cfg(feature = "fhir")]
pub use tpt_io_fhir as fhir;
#[cfg(feature = "fits")]
pub use tpt_io_fits as fits;
#[cfg(feature = "gbxml")]
pub use tpt_io_gbxml as gbxml;
#[cfg(feature = "gcode")]
pub use tpt_io_gcode as gcode;
#[cfg(feature = "gds")]
pub use tpt_io_gds as gds;
#[cfg(feature = "geo")]
pub use tpt_io_geo as geo;
#[cfg(feature = "gerber")]
pub use tpt_io_gerber as gerber;
#[cfg(feature = "gnss")]
pub use tpt_io_gnss as gnss;
#[cfg(feature = "grib")]
pub use tpt_io_grib as grib;
#[cfg(feature = "gridlabd")]
pub use tpt_io_gridlabd as gridlabd;
#[cfg(feature = "hdf5")]
pub use tpt_io_hdf5 as hdf5;
#[cfg(feature = "hecras")]
pub use tpt_io_hecras as hecras;
#[cfg(feature = "hl7v2")]
pub use tpt_io_hl7v2 as hl7v2;
#[cfg(feature = "ifc")]
pub use tpt_io_ifc as ifc;
#[cfg(feature = "ipc2581")]
pub use tpt_io_ipc2581 as ipc2581;
#[cfg(feature = "jcamp")]
pub use tpt_io_jcamp as jcamp;
#[cfg(feature = "kicad")]
pub use tpt_io_kicad as kicad;
#[cfg(feature = "landxml")]
pub use tpt_io_landxml as landxml;
#[cfg(feature = "matpower")]
pub use tpt_io_matpower as matpower;
#[cfg(feature = "mcap")]
pub use tpt_io_mcap as mcap;
#[cfg(feature = "mdf")]
pub use tpt_io_mdf as mdf;
#[cfg(feature = "mesh")]
pub use tpt_io_mesh as mesh;
#[cfg(feature = "mtconnect")]
pub use tpt_io_mtconnect as mtconnect;
#[cfg(feature = "mzml")]
pub use tpt_io_mzml as mzml;
#[cfg(feature = "naif")]
pub use tpt_io_naif as naif;
#[cfg(feature = "netcdf")]
pub use tpt_io_netcdf as netcdf;
#[cfg(feature = "nifti")]
pub use tpt_io_nifti as nifti;
#[cfg(feature = "nrrd")]
pub use tpt_io_nrrd as nrrd;
#[cfg(feature = "ome")]
pub use tpt_io_ome as ome;
#[cfg(feature = "omex")]
pub use tpt_io_omex as omex;
#[cfg(feature = "opcua")]
pub use tpt_io_opcua as opcua;
#[cfg(feature = "opendrive")]
pub use tpt_io_opendrive as opendrive;
#[cfg(feature = "opendss")]
pub use tpt_io_opendss as opendss;
#[cfg(feature = "openehr")]
pub use tpt_io_openehr as openehr;
#[cfg(feature = "openscenario")]
pub use tpt_io_openscenario as openscenario;
#[cfg(feature = "pdb")]
pub use tpt_io_pdb as pdb;
#[cfg(feature = "plcopen")]
pub use tpt_io_plcopen as plcopen;
#[cfg(feature = "pointcloud")]
pub use tpt_io_pointcloud as pointcloud;
#[cfg(feature = "rosbag")]
pub use tpt_io_rosbag as rosbag;
#[cfg(feature = "sbml")]
pub use tpt_io_sbml as sbml;
#[cfg(feature = "segy")]
pub use tpt_io_segy as segy;
#[cfg(feature = "spice")]
pub use tpt_io_spice as spice;
#[cfg(feature = "threemf")]
pub use tpt_io_threemf as threemf;
#[cfg(feature = "tiff")]
pub use tpt_io_tiff as tiff;
#[cfg(feature = "tle")]
pub use tpt_io_tle as tle;
#[cfg(feature = "touchstone")]
pub use tpt_io_touchstone as touchstone;
#[cfg(feature = "urdf")]
pub use tpt_io_urdf as urdf;
#[cfg(feature = "verilog")]
pub use tpt_io_verilog as verilog;
#[cfg(feature = "waterml")]
pub use tpt_io_waterml as waterml;
#[cfg(feature = "welllog")]
pub use tpt_io_welllog as welllog;
#[cfg(feature = "witsml")]
pub use tpt_io_witsml as witsml;
