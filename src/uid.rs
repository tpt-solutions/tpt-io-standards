//! UID registry for well-known DICOM SOP classes and transfer syntaxes
//! (PS3.6 Table A-1, Table 8-1, Table A-2).

/// Well-known DICOM SOP Class UIDs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SopClass {
    /// Verification SOP Class
    Verification,
    /// CT Image Storage
    CtImageStorage,
    /// MR Image Storage
    MrImageStorage,
    /// Computed Radiography Image Storage
    ComputedRadiographyImageStorage,
    /// Digital X-Ray Image Storage - For Presentation
    DigitalXRayImageStorageForPresentation,
    /// Digital X-Ray Image Storage - For Processing
    DigitalXRayImageStorageForProcessing,
    /// Digital Mammography X-Ray Image Storage - For Presentation
    DigitalMammographyXRayImageStorageForPresentation,
    /// Digital Mammography X-Ray Image Storage - For Processing
    DigitalMammographyXRayImageStorageForProcessing,
    /// Ultrasound Multi-frame Image Storage (Retired)
    UltrasoundMultiFrameImageStorageRetired,
    /// Ultrasound Multi-frame Image Storage
    UltrasoundMultiFrameImageStorage,
    /// Secondary Capture Image Storage
    SecondaryCaptureImageStorage,
    /// Multi-frame Single Bit Secondary Capture Image Storage
    MultiFrameSingleBitSecondaryCaptureImageStorage,
    /// Multi-frame Grayscale Byte Secondary Capture Image Storage
    MultiFrameGrayscaleByteSecondaryCaptureImageStorage,
    /// Multi-frame Grayscale Word Secondary Capture Image Storage
    MultiFrameGrayscaleWordSecondaryCaptureImageStorage,
    /// Multi-frame True Color Secondary Capture Image Storage
    MultiFrameTrueColorSecondaryCaptureImageStorage,
    /// Encapsulated PDF Storage
    EncapsulatedPdf,
    /// RT Image Storage
    RtImageStorage,
    /// RT Dose Storage
    RtDoseStorage,
    /// RT Structure Set Storage
    RtStructureSetStorage,
    /// RT Beams Treatment Record Storage
    RtBeamsTreatmentRecordStorage,
    /// RT Plan Storage
    RtPlanStorage,
    /// RT Brachy Treatment Record Storage
    RtBrachyTreatmentRecordStorage,
    /// RT Treatment Summary Record Storage
    RtTreatmentSummaryRecordStorage,
    /// Nuclear Medicine Image Storage
    NuclearMedicineImageStorage,
    /// Positron Emission Tomography Image Storage
    PetImageStorage,
    /// Parametric Map Storage
    ParametricMap,
    /// Segmentation Storage
    Segmentation,
    /// Surface Segmentation Storage
    SurfaceSegmentation,
    /// Basic Text SR Storage
    BasicTextSr,
    /// Enhanced SR Storage
    EnhancedSr,
    /// Comprehensive SR Storage
    ComprehensiveSr,
    /// Key Object Selection Document Storage
    KeyObjectSelection,
    /// Chest CAD SR Storage
    ChestCadSr,
    /// X-Ray Radiation Dose SR Storage
    XRayRadiationDoseSr,
    /// Mammography CAD SR Storage
    MammographyCadSr,
    /// Procedure Log Storage
    ProcedureLog,
    /// TLSend
    TlSend,
}

impl SopClass {
    /// The UID string for this SOP class.
    pub const fn uid(self) -> &'static str {
        match self {
            SopClass::Verification => "1.2.840.10008.1.1",
            SopClass::CtImageStorage => "1.2.840.10008.5.1.4.1.1.2",
            SopClass::MrImageStorage => "1.2.840.10008.5.1.4.1.1.4",
            SopClass::ComputedRadiographyImageStorage => "1.2.840.10008.5.1.4.1.1.1",
            SopClass::DigitalXRayImageStorageForPresentation => "1.2.840.10008.5.1.4.1.1.1.1",
            SopClass::DigitalXRayImageStorageForProcessing => "1.2.840.10008.5.1.4.1.1.1.1.1",
            SopClass::DigitalMammographyXRayImageStorageForPresentation => "1.2.840.10008.5.1.4.1.1.1.2",
            SopClass::DigitalMammographyXRayImageStorageForProcessing => "1.2.840.10008.5.1.4.1.1.1.2.1",
            SopClass::UltrasoundMultiFrameImageStorageRetired => "1.2.840.10008.5.1.4.1.1.3",
            SopClass::UltrasoundMultiFrameImageStorage => "1.2.840.10008.5.1.4.1.1.3.1",
            SopClass::SecondaryCaptureImageStorage => "1.2.840.10008.5.1.4.1.1.7",
            SopClass::MultiFrameSingleBitSecondaryCaptureImageStorage => "1.2.840.10008.5.1.4.1.1.7.1",
            SopClass::MultiFrameGrayscaleByteSecondaryCaptureImageStorage => "1.2.840.10008.5.1.4.1.1.7.2",
            SopClass::MultiFrameGrayscaleWordSecondaryCaptureImageStorage => "1.2.840.10008.5.1.4.1.1.7.3",
            SopClass::MultiFrameTrueColorSecondaryCaptureImageStorage => "1.2.840.10008.5.1.4.1.1.7.4",
            SopClass::EncapsulatedPdf => "1.2.840.10008.5.1.4.1.1.104.1",
            SopClass::RtImageStorage => "1.2.840.10008.5.1.4.1.1.481.1",
            SopClass::RtDoseStorage => "1.2.840.10008.5.1.4.1.1.481.2",
            SopClass::RtStructureSetStorage => "1.2.840.10008.5.1.4.1.1.481.3",
            SopClass::RtBeamsTreatmentRecordStorage => "1.2.840.10008.5.1.4.1.1.481.4",
            SopClass::RtPlanStorage => "1.2.840.10008.5.1.4.1.1.481.5",
            SopClass::RtBrachyTreatmentRecordStorage => "1.2.840.10008.5.1.4.1.1.481.6",
            SopClass::RtTreatmentSummaryRecordStorage => "1.2.840.10008.5.1.4.1.1.481.7",
            SopClass::NuclearMedicineImageStorage => "1.2.840.10008.5.1.4.1.1.20",
            SopClass::PetImageStorage => "1.2.840.10008.5.1.4.1.1.128",
            SopClass::ParametricMap => "1.2.840.10008.5.1.4.1.1.30",
            SopClass::Segmentation => "1.2.840.10008.5.1.4.1.1.66.4",
            SopClass::SurfaceSegmentation => "1.2.840.10008.5.1.4.1.1.66.5",
            SopClass::BasicTextSr => "1.2.840.10008.5.1.4.1.1.88.11",
            SopClass::EnhancedSr => "1.2.840.10008.5.1.4.1.1.88.22",
            SopClass::ComprehensiveSr => "1.2.840.10008.5.1.4.1.1.88.33",
            SopClass::KeyObjectSelection => "1.2.840.10008.5.1.4.1.1.88.59",
            SopClass::ChestCadSr => "1.2.840.10008.5.1.4.1.1.88.65",
            SopClass::XRayRadiationDoseSr => "1.2.840.10008.5.1.4.1.1.88.67",
            SopClass::MammographyCadSr => "1.2.840.10008.5.1.4.1.1.88.69",
            SopClass::ProcedureLog => "1.2.840.10008.5.1.4.1.1.88.40",
            SopClass::TlSend => "1.2.840.10008.5.1.4.1.1.77.1.6",
        }
    }

    /// Look up a SOP class by its UID string.
    pub fn from_uid(uid: &str) -> Option<Self> {
        match uid {
            "1.2.840.10008.1.1" => Some(SopClass::Verification),
            "1.2.840.10008.5.1.4.1.1.2" => Some(SopClass::CtImageStorage),
            "1.2.840.10008.5.1.4.1.1.4" => Some(SopClass::MrImageStorage),
            "1.2.840.10008.5.1.4.1.1.1" => Some(SopClass::ComputedRadiographyImageStorage),
            "1.2.840.10008.5.1.4.1.1.1.1" => Some(SopClass::DigitalXRayImageStorageForPresentation),
            "1.2.840.10008.5.1.4.1.1.1.1.1" => Some(SopClass::DigitalXRayImageStorageForProcessing),
            "1.2.840.10008.5.1.4.1.1.1.2" => Some(SopClass::DigitalMammographyXRayImageStorageForPresentation),
            "1.2.840.10008.5.1.4.1.1.1.2.1" => Some(SopClass::DigitalMammographyXRayImageStorageForProcessing),
            "1.2.840.10008.5.1.4.1.1.3" => Some(SopClass::UltrasoundMultiFrameImageStorageRetired),
            "1.2.840.10008.5.1.4.1.1.3.1" => Some(SopClass::UltrasoundMultiFrameImageStorage),
            "1.2.840.10008.5.1.4.1.1.7" => Some(SopClass::SecondaryCaptureImageStorage),
            "1.2.840.10008.5.1.4.1.1.7.1" => Some(SopClass::MultiFrameSingleBitSecondaryCaptureImageStorage),
            "1.2.840.10008.5.1.4.1.1.7.2" => Some(SopClass::MultiFrameGrayscaleByteSecondaryCaptureImageStorage),
            "1.2.840.10008.5.1.4.1.1.7.3" => Some(SopClass::MultiFrameGrayscaleWordSecondaryCaptureImageStorage),
            "1.2.840.10008.5.1.4.1.1.7.4" => Some(SopClass::MultiFrameTrueColorSecondaryCaptureImageStorage),
            "1.2.840.10008.5.1.4.1.1.104.1" => Some(SopClass::EncapsulatedPdf),
            "1.2.840.10008.5.1.4.1.1.481.1" => Some(SopClass::RtImageStorage),
            "1.2.840.10008.5.1.4.1.1.481.2" => Some(SopClass::RtDoseStorage),
            "1.2.840.10008.5.1.4.1.1.481.3" => Some(SopClass::RtStructureSetStorage),
            "1.2.840.10008.5.1.4.1.1.481.4" => Some(SopClass::RtBeamsTreatmentRecordStorage),
            "1.2.840.10008.5.1.4.1.1.481.5" => Some(SopClass::RtPlanStorage),
            "1.2.840.10008.5.1.4.1.1.481.6" => Some(SopClass::RtBrachyTreatmentRecordStorage),
            "1.2.840.10008.5.1.4.1.1.481.7" => Some(SopClass::RtTreatmentSummaryRecordStorage),
            "1.2.840.10008.5.1.4.1.1.20" => Some(SopClass::NuclearMedicineImageStorage),
            "1.2.840.10008.5.1.4.1.1.128" => Some(SopClass::PetImageStorage),
            "1.2.840.10008.5.1.4.1.1.30" => Some(SopClass::ParametricMap),
            "1.2.840.10008.5.1.4.1.1.66.4" => Some(SopClass::Segmentation),
            "1.2.840.10008.5.1.4.1.1.66.5" => Some(SopClass::SurfaceSegmentation),
            "1.2.840.10008.5.1.4.1.1.88.11" => Some(SopClass::BasicTextSr),
            "1.2.840.10008.5.1.4.1.1.88.22" => Some(SopClass::EnhancedSr),
            "1.2.840.10008.5.1.4.1.1.88.33" => Some(SopClass::ComprehensiveSr),
            "1.2.840.10008.5.1.4.1.1.88.59" => Some(SopClass::KeyObjectSelection),
            "1.2.840.10008.5.1.4.1.1.88.65" => Some(SopClass::ChestCadSr),
            "1.2.840.10008.5.1.4.1.1.88.67" => Some(SopClass::XRayRadiationDoseSr),
            "1.2.840.10008.5.1.4.1.1.88.69" => Some(SopClass::MammographyCadSr),
            "1.2.840.10008.5.1.4.1.1.88.40" => Some(SopClass::ProcedureLog),
            "1.2.840.10008.5.1.4.1.1.77.1.6" => Some(SopClass::TlSend),
            _ => None,
        }
    }
}

/// Well-known DICOM Transfer Syntax UIDs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TransferSyntax {
    /// Implicit VR Little Endian
    ImplicitVrLittleEndian,
    /// Explicit VR Little Endian
    ExplicitVrLittleEndian,
    /// Explicit VR Big Endian (Retired)
    ExplicitVrBigEndian,
    /// JPEG Baseline (Process 1)
    JpegBaseline,
    /// JPEG Extended (Process 2 & 4)
    JpegExtended,
    /// JPEG Lossless, Non-Hierarchical (Process 14)
    JpegLossless,
    /// JPEG 2000 Image Compression (Lossless Only)
    Jpeg2000Lossless,
    /// JPEG 2000 Image Compression
    Jpeg2000,
    /// RLE Lossless
    RleLossless,
    /// MPEG2 Main Profile / Main Level
    Mpeg2,
    /// MPEG4 AVC/H.264 High Profile
    Mpeg4,
    /// Deflated Explicit VR Little Endian
    DeflatedExplicitVrLittleEndian,
    /// JPIP Referenced
    JpipReferenced,
    /// JPIP Referenced Deflate
    JpipReferencedDeflate,
}

impl TransferSyntax {
    /// The UID string for this transfer syntax.
    pub const fn uid(self) -> &'static str {
        match self {
            TransferSyntax::ImplicitVrLittleEndian => "1.2.840.10008.1.2",
            TransferSyntax::ExplicitVrLittleEndian => "1.2.840.10008.1.2.1",
            TransferSyntax::ExplicitVrBigEndian => "1.2.840.10008.1.2.2",
            TransferSyntax::JpegBaseline => "1.2.840.10008.1.2.4.50",
            TransferSyntax::JpegExtended => "1.2.840.10008.1.2.4.51",
            TransferSyntax::JpegLossless => "1.2.840.10008.1.2.4.70",
            TransferSyntax::Jpeg2000Lossless => "1.2.840.10008.1.2.4.90",
            TransferSyntax::Jpeg2000 => "1.2.840.10008.1.2.4.91",
            TransferSyntax::RleLossless => "1.2.840.10008.1.2.5",
            TransferSyntax::Mpeg2 => "1.2.840.10008.1.2.4.100",
            TransferSyntax::Mpeg4 => "1.2.840.10008.1.2.4.102",
            TransferSyntax::DeflatedExplicitVrLittleEndian => "1.2.840.10008.1.2.1.99",
            TransferSyntax::JpipReferenced => "1.2.840.10008.1.2.4.94",
            TransferSyntax::JpipReferencedDeflate => "1.2.840.10008.1.2.4.95",
        }
    }

    /// Look up a transfer syntax by its UID string.
    pub fn from_uid(uid: &str) -> Option<Self> {
        match uid {
            "1.2.840.10008.1.2" => Some(TransferSyntax::ImplicitVrLittleEndian),
            "1.2.840.10008.1.2.1" => Some(TransferSyntax::ExplicitVrLittleEndian),
            "1.2.840.10008.1.2.2" => Some(TransferSyntax::ExplicitVrBigEndian),
            "1.2.840.10008.1.2.4.50" => Some(TransferSyntax::JpegBaseline),
            "1.2.840.10008.1.2.4.51" => Some(TransferSyntax::JpegExtended),
            "1.2.840.10008.1.2.4.70" => Some(TransferSyntax::JpegLossless),
            "1.2.840.10008.1.2.4.90" => Some(TransferSyntax::Jpeg2000Lossless),
            "1.2.840.10008.1.2.4.91" => Some(TransferSyntax::Jpeg2000),
            "1.2.840.10008.1.2.5" => Some(TransferSyntax::RleLossless),
            "1.2.840.10008.1.2.4.100" => Some(TransferSyntax::Mpeg2),
            "1.2.840.10008.1.2.4.102" => Some(TransferSyntax::Mpeg4),
            "1.2.840.10008.1.2.1.99" => Some(TransferSyntax::DeflatedExplicitVrLittleEndian),
            "1.2.840.10008.1.2.4.94" => Some(TransferSyntax::JpipReferenced),
            "1.2.840.10008.1.2.4.95" => Some(TransferSyntax::JpipReferencedDeflate),
            _ => None,
        }
    }

    /// Whether this transfer syntax uses implicit VR encoding.
    pub const fn is_implicit_vr(self) -> bool {
        matches!(self, TransferSyntax::ImplicitVrLittleEndian)
    }

    /// Whether this transfer syntax uses explicit VR encoding.
    pub const fn is_explicit_vr(self) -> bool {
        !self.is_implicit_vr()
    }

    /// Whether this transfer syntax uses little-endian byte order.
    pub const fn is_little_endian(self) -> bool {
        !matches!(self, TransferSyntax::ExplicitVrBigEndian)
    }

    /// Whether this transfer syntax implies compressed pixel data.
    pub const fn is_compressed(self) -> bool {
        matches!(
            self,
            TransferSyntax::JpegBaseline
                | TransferSyntax::JpegExtended
                | TransferSyntax::JpegLossless
                | TransferSyntax::Jpeg2000Lossless
                | TransferSyntax::Jpeg2000
                | TransferSyntax::RleLossless
                | TransferSyntax::Mpeg2
                | TransferSyntax::Mpeg4
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sop_class_round_trip() {
        for sop in [SopClass::CtImageStorage, SopClass::MrImageStorage, SopClass::Verification] {
            assert_eq!(SopClass::from_uid(sop.uid()), Some(sop));
        }
    }

    #[test]
    fn unknown_sop_returns_none() {
        assert!(SopClass::from_uid("1.2.3.4.5").is_none());
    }

    #[test]
    fn transfer_syntax_round_trip() {
        for ts in [
            TransferSyntax::ImplicitVrLittleEndian,
            TransferSyntax::ExplicitVrLittleEndian,
            TransferSyntax::Jpeg2000,
            TransferSyntax::RleLossless,
        ] {
            assert_eq!(TransferSyntax::from_uid(ts.uid()), Some(ts));
        }
    }

    #[test]
    fn transfer_syntax_flags() {
        assert!(TransferSyntax::ImplicitVrLittleEndian.is_implicit_vr());
        assert!(!TransferSyntax::ExplicitVrLittleEndian.is_implicit_vr());
        assert!(TransferSyntax::Jpeg2000.is_compressed());
        assert!(!TransferSyntax::ImplicitVrLittleEndian.is_compressed());
        assert!(TransferSyntax::ExplicitVrLittleEndian.is_little_endian());
        assert!(!TransferSyntax::ExplicitVrBigEndian.is_little_endian());
    }
}
