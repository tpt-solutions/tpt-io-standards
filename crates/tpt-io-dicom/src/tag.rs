//! Standard DICOM tag dictionary (PS3.6).
//!
//! Provides a lookup from a `(group, element)` pair to the tag's keyword,
//! Value Representation (VR), and VM (Value Multiplicity). The dictionary
//! covers the most commonly encountered tags in clinical datasets; it is not
//! exhaustive (the full registry has thousands of entries).

/// A DICOM tag's metadata: keyword, VR, and VM descriptor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TagInfo {
    /// The DICOM keyword, e.g. `"SOPClassUID"`.
    pub keyword: &'static str,
    /// The standard VR(s) for this tag.
    pub vr: &'static [VRCode],
    /// Value Multiplicity, e.g. `"1"`, `"2-2"``, `"1-n"`.
    pub vm: &'static str,
}

/// DICOM Value Representation (VR) codes (PS3.5 §6.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum VRCode {
    /// Application Entity
    AE,
    /// Age String
    AS,
    /// Attribute Tag
    AT,
    /// Code String
    CS,
    /// Date
    DA,
    /// Decimal String
    DS,
    /// Date Time
    DT,
    /// Floating Point Single
    FL,
    /// Floating Point Double
    FD,
    /// Integer String
    IS,
    /// Long String
    LO,
    /// Long Text
    LT,
    /// Other Byte
    OB,
    /// Other Double
    OD,
    /// Other Float
    OF,
    /// Other Long
    OL,
    /// Other 64-bit Very Long
    OV,
    /// Other Word
    OW,
    /// Person Name
    PN,
    /// Short String
    SH,
    /// Signed Long
    SL,
    /// Sequence of Items
    SQ,
    /// Signed Short
    SS,
    /// Short Text
    ST,
    /// Time
    TM,
    /// Unlimited Characters
    UC,
    /// Unique Identifier
    UI,
    /// Unsigned Long
    UL,
    /// Unknown
    UN,
    /// Universal Resource / Uniform Resource Identifier / URL
    UR,
    /// Unsigned Short
    US,
    /// Unlimited Text
    UT,
}

impl VRCode {
    /// Two-byte ASCII representation of this VR code.
    pub const fn as_bytes(self) -> [u8; 2] {
        match self {
            VRCode::AE => *b"AE",
            VRCode::AS => *b"AS",
            VRCode::AT => *b"AT",
            VRCode::CS => *b"CS",
            VRCode::DA => *b"DA",
            VRCode::DS => *b"DS",
            VRCode::DT => *b"DT",
            VRCode::FL => *b"FL",
            VRCode::FD => *b"FD",
            VRCode::IS => *b"IS",
            VRCode::LO => *b"LO",
            VRCode::LT => *b"LT",
            VRCode::OB => *b"OB",
            VRCode::OD => *b"OD",
            VRCode::OF => *b"OF",
            VRCode::OL => *b"OL",
            VRCode::OV => *b"OV",
            VRCode::OW => *b"OW",
            VRCode::PN => *b"PN",
            VRCode::SH => *b"SH",
            VRCode::SL => *b"SL",
            VRCode::SQ => *b"SQ",
            VRCode::SS => *b"SS",
            VRCode::ST => *b"ST",
            VRCode::TM => *b"TM",
            VRCode::UC => *b"UC",
            VRCode::UI => *b"UI",
            VRCode::UL => *b"UL",
            VRCode::UN => *b"UN",
            VRCode::UR => *b"UR",
            VRCode::US => *b"US",
            VRCode::UT => *b"UT",
        }
    }

    /// Parse a VR code from its two-byte ASCII representation.
    pub const fn from_bytes(b: [u8; 2]) -> Option<Self> {
        match &b {
            b"AE" => Some(VRCode::AE),
            b"AS" => Some(VRCode::AS),
            b"AT" => Some(VRCode::AT),
            b"CS" => Some(VRCode::CS),
            b"DA" => Some(VRCode::DA),
            b"DS" => Some(VRCode::DS),
            b"DT" => Some(VRCode::DT),
            b"FL" => Some(VRCode::FL),
            b"FD" => Some(VRCode::FD),
            b"IS" => Some(VRCode::IS),
            b"LO" => Some(VRCode::LO),
            b"LT" => Some(VRCode::LT),
            b"OB" => Some(VRCode::OB),
            b"OD" => Some(VRCode::OD),
            b"OF" => Some(VRCode::OF),
            b"OL" => Some(VRCode::OL),
            b"OV" => Some(VRCode::OV),
            b"OW" => Some(VRCode::OW),
            b"PN" => Some(VRCode::PN),
            b"SH" => Some(VRCode::SH),
            b"SL" => Some(VRCode::SL),
            b"SQ" => Some(VRCode::SQ),
            b"SS" => Some(VRCode::SS),
            b"ST" => Some(VRCode::ST),
            b"TM" => Some(VRCode::TM),
            b"UC" => Some(VRCode::UC),
            b"UI" => Some(VRCode::UI),
            b"UL" => Some(VRCode::UL),
            b"UN" => Some(VRCode::UN),
            b"UR" => Some(VRCode::UR),
            b"US" => Some(VRCode::US),
            b"UT" => Some(VRCode::UT),
            _ => None,
        }
    }

    /// The maximum byte length of a single value element for this VR.
    /// Returns `None` for variable-length VRs (OB, OW, OD, OF, OL, OV, SQ, UN, UC, UR, UT).
    pub const fn max_element_len(self) -> Option<usize> {
        match self {
            VRCode::AE => Some(16),
            VRCode::AS => Some(4),
            VRCode::AT => Some(4),
            VRCode::CS => Some(16),
            VRCode::DA => Some(8),
            VRCode::DS => Some(16),
            VRCode::DT => Some(26),
            VRCode::FL => Some(4),
            VRCode::FD => Some(8),
            VRCode::IS => Some(12),
            VRCode::LO => Some(64),
            VRCode::LT => Some(10240),
            VRCode::OB => None,
            VRCode::OD => None,
            VRCode::OF => None,
            VRCode::OL => None,
            VRCode::OV => None,
            VRCode::OW => None,
            VRCode::PN => Some(64),
            VRCode::SH => Some(16),
            VRCode::SL => Some(4),
            VRCode::SQ => None,
            VRCode::SS => Some(2),
            VRCode::ST => Some(1024),
            VRCode::TM => Some(16),
            VRCode::UC => None,
            VRCode::UI => Some(64),
            VRCode::UL => Some(4),
            VRCode::UN => None,
            VRCode::UR => None,
            VRCode::US => Some(2),
            VRCode::UT => None,
        }
    }
}

/// Look up tag metadata by `(group, element)`.
///
/// Returns `None` for private or unrecognized tags.
pub fn lookup_tag(group: u16, element: u16) -> Option<TagInfo> {
    // Standard DICOM tags — subset of PS3.6.
    // Organized by group for readability.
    if group == 0x0002 {
        match element {
            0x0000 => {
                return Some(TagInfo {
                    keyword: "FileMetaInformationGroupLength",
                    vr: &[VRCode::UL],
                    vm: "1",
                })
            }
            0x0001 => {
                return Some(TagInfo {
                    keyword: "FileMetaInformationVersion",
                    vr: &[VRCode::OB],
                    vm: "1",
                })
            }
            0x0002 => {
                return Some(TagInfo {
                    keyword: "MediaStorageSOPClassUID",
                    vr: &[VRCode::UI],
                    vm: "1",
                })
            }
            0x0003 => {
                return Some(TagInfo {
                    keyword: "MediaStorageSOPInstanceUID",
                    vr: &[VRCode::UI],
                    vm: "1",
                })
            }
            0x0010 => {
                return Some(TagInfo { keyword: "TransferSyntaxUID", vr: &[VRCode::UI], vm: "1" })
            }
            0x0012 => {
                return Some(TagInfo {
                    keyword: "ImplementationClassUID",
                    vr: &[VRCode::UI],
                    vm: "1",
                })
            }
            0x0013 => {
                return Some(TagInfo {
                    keyword: "ImplementationVersionName",
                    vr: &[VRCode::SH],
                    vm: "1",
                })
            }
            0x0016 => {
                return Some(TagInfo {
                    keyword: "SourceApplicationEntityTitle",
                    vr: &[VRCode::AE],
                    vm: "1",
                })
            }
            0x0100 => {
                return Some(TagInfo {
                    keyword: "PrivateInformationCreatorUID",
                    vr: &[VRCode::UI],
                    vm: "1",
                })
            }
            0x0102 => {
                return Some(TagInfo { keyword: "PrivateInformation", vr: &[VRCode::OB], vm: "1" })
            }
            _ => {}
        }
    }
    if group == 0x0008 {
        match element {
            0x0005 => {
                return Some(TagInfo {
                    keyword: "SpecificCharacterSet",
                    vr: &[VRCode::CS],
                    vm: "1-n",
                })
            }
            0x0008 => return Some(TagInfo { keyword: "ImageType", vr: &[VRCode::CS], vm: "2-n" }),
            0x0012 => {
                return Some(TagInfo {
                    keyword: "InstanceCreationDate",
                    vr: &[VRCode::DA],
                    vm: "1",
                })
            }
            0x0013 => {
                return Some(TagInfo {
                    keyword: "InstanceCreationTime",
                    vr: &[VRCode::TM],
                    vm: "1",
                })
            }
            0x0014 => {
                return Some(TagInfo { keyword: "InstanceCreatorUID", vr: &[VRCode::UI], vm: "1" })
            }
            0x0016 => return Some(TagInfo { keyword: "SOPClassUID", vr: &[VRCode::UI], vm: "1" }),
            0x0018 => {
                return Some(TagInfo { keyword: "SOPInstanceUID", vr: &[VRCode::UI], vm: "1" })
            }
            0x0020 => return Some(TagInfo { keyword: "StudyDate", vr: &[VRCode::DA], vm: "1" }),
            0x0021 => return Some(TagInfo { keyword: "SeriesDate", vr: &[VRCode::DA], vm: "1" }),
            0x0022 => {
                return Some(TagInfo { keyword: "AcquisitionDate", vr: &[VRCode::DA], vm: "1" })
            }
            0x0023 => return Some(TagInfo { keyword: "ContentDate", vr: &[VRCode::DA], vm: "1" }),
            0x0030 => return Some(TagInfo { keyword: "StudyTime", vr: &[VRCode::TM], vm: "1" }),
            0x0031 => return Some(TagInfo { keyword: "SeriesTime", vr: &[VRCode::TM], vm: "1" }),
            0x0032 => {
                return Some(TagInfo { keyword: "AcquisitionTime", vr: &[VRCode::TM], vm: "1" })
            }
            0x0033 => return Some(TagInfo { keyword: "ContentTime", vr: &[VRCode::TM], vm: "1" }),
            0x0050 => {
                return Some(TagInfo { keyword: "AccessionNumber", vr: &[VRCode::SH], vm: "1" })
            }
            0x0060 => return Some(TagInfo { keyword: "Modality", vr: &[VRCode::CS], vm: "1" }),
            0x0061 => {
                return Some(TagInfo { keyword: "ModalitiesInStudy", vr: &[VRCode::CS], vm: "1-n" })
            }
            0x0062 => {
                return Some(TagInfo { keyword: "SOPClassesInStudy", vr: &[VRCode::UI], vm: "1-n" })
            }
            0x0070 => return Some(TagInfo { keyword: "Manufacturer", vr: &[VRCode::LO], vm: "1" }),
            0x0080 => {
                return Some(TagInfo { keyword: "InstitutionName", vr: &[VRCode::LO], vm: "1" })
            }
            0x0081 => {
                return Some(TagInfo { keyword: "InstitutionAddress", vr: &[VRCode::ST], vm: "1" })
            }
            0x0090 => {
                return Some(TagInfo {
                    keyword: "ReferringPhysicianName",
                    vr: &[VRCode::PN],
                    vm: "1",
                })
            }
            0x0100 => return Some(TagInfo { keyword: "CodeValue", vr: &[VRCode::SH], vm: "1" }),
            0x0102 => {
                return Some(TagInfo {
                    keyword: "CodingSchemeDesignator",
                    vr: &[VRCode::SH],
                    vm: "1",
                })
            }
            0x0104 => return Some(TagInfo { keyword: "CodeMeaning", vr: &[VRCode::LO], vm: "1" }),
            0x1010 => return Some(TagInfo { keyword: "StationName", vr: &[VRCode::SH], vm: "1" }),
            0x1030 => {
                return Some(TagInfo { keyword: "StudyDescription", vr: &[VRCode::LO], vm: "1" })
            }
            0x103E => {
                return Some(TagInfo { keyword: "SeriesDescription", vr: &[VRCode::LO], vm: "1" })
            }
            0x1048 => {
                return Some(TagInfo {
                    keyword: "PhysiciansOfRecord",
                    vr: &[VRCode::PN],
                    vm: "1-n",
                })
            }
            0x1060 => {
                return Some(TagInfo {
                    keyword: "NameOfPhysiciansReadingStudy",
                    vr: &[VRCode::PN],
                    vm: "1-n",
                })
            }
            0x1090 => {
                return Some(TagInfo {
                    keyword: "ManufacturerModelName",
                    vr: &[VRCode::LO],
                    vm: "1",
                })
            }
            0x2111 => {
                return Some(TagInfo {
                    keyword: "DerivationDescription",
                    vr: &[VRCode::ST],
                    vm: "1",
                })
            }
            _ => {}
        }
    }
    if group == 0x0010 {
        match element {
            0x0010 => return Some(TagInfo { keyword: "PatientName", vr: &[VRCode::PN], vm: "1" }),
            0x0020 => return Some(TagInfo { keyword: "PatientID", vr: &[VRCode::LO], vm: "1" }),
            0x0021 => {
                return Some(TagInfo { keyword: "IssuerOfPatientID", vr: &[VRCode::LO], vm: "1" })
            }
            0x0030 => {
                return Some(TagInfo { keyword: "PatientBirthDate", vr: &[VRCode::DA], vm: "1" })
            }
            0x0032 => {
                return Some(TagInfo { keyword: "PatientBirthTime", vr: &[VRCode::TM], vm: "1" })
            }
            0x0040 => return Some(TagInfo { keyword: "PatientSex", vr: &[VRCode::CS], vm: "1" }),
            0x1000 => {
                return Some(TagInfo { keyword: "OtherPatientIDs", vr: &[VRCode::LO], vm: "1-n" })
            }
            0x1001 => {
                return Some(TagInfo { keyword: "OtherPatientNames", vr: &[VRCode::PN], vm: "1-n" })
            }
            0x1010 => return Some(TagInfo { keyword: "PatientAge", vr: &[VRCode::AS], vm: "1" }),
            0x1020 => return Some(TagInfo { keyword: "PatientSize", vr: &[VRCode::DS], vm: "1" }),
            0x1030 => {
                return Some(TagInfo { keyword: "PatientWeight", vr: &[VRCode::DS], vm: "1" })
            }
            _ => {}
        }
    }
    if group == 0x0018 {
        match element {
            0x0015 => {
                return Some(TagInfo { keyword: "BodyPartExamined", vr: &[VRCode::CS], vm: "1" })
            }
            0x0020 => {
                return Some(TagInfo { keyword: "ScanningSequence", vr: &[VRCode::CS], vm: "1-n" })
            }
            0x0021 => {
                return Some(TagInfo { keyword: "SequenceVariant", vr: &[VRCode::CS], vm: "1-n" })
            }
            0x0022 => {
                return Some(TagInfo { keyword: "ScanOptions", vr: &[VRCode::CS], vm: "1-n" })
            }
            0x0023 => {
                return Some(TagInfo { keyword: "MRAcquisitionType", vr: &[VRCode::CS], vm: "1" })
            }
            0x0050 => {
                return Some(TagInfo { keyword: "SliceThickness", vr: &[VRCode::DS], vm: "1" })
            }
            0x0060 => return Some(TagInfo { keyword: "KVP", vr: &[VRCode::DS], vm: "1" }),
            0x0080 => {
                return Some(TagInfo { keyword: "RepetitionTime", vr: &[VRCode::DS], vm: "1" })
            }
            0x0081 => return Some(TagInfo { keyword: "EchoTime", vr: &[VRCode::DS], vm: "1" }),
            0x0087 => {
                return Some(TagInfo {
                    keyword: "MagneticFieldStrength",
                    vr: &[VRCode::DS],
                    vm: "1",
                })
            }
            0x0091 => {
                return Some(TagInfo { keyword: "EchoTrainLength", vr: &[VRCode::IS], vm: "1" })
            }
            0x1000 => {
                return Some(TagInfo { keyword: "DeviceSerialNumber", vr: &[VRCode::LO], vm: "1" })
            }
            0x1020 => {
                return Some(TagInfo { keyword: "SoftwareVersions", vr: &[VRCode::LO], vm: "1-n" })
            }
            0x1030 => return Some(TagInfo { keyword: "ProtocolName", vr: &[VRCode::LO], vm: "1" }),
            0x1100 => {
                return Some(TagInfo {
                    keyword: "ReconstructionDiameter",
                    vr: &[VRCode::DS],
                    vm: "1",
                })
            }
            0x1310 => {
                return Some(TagInfo { keyword: "AcquisitionMatrix", vr: &[VRCode::US], vm: "4" })
            }
            0x5100 => {
                return Some(TagInfo { keyword: "PatientPosition", vr: &[VRCode::CS], vm: "1" })
            }
            _ => {}
        }
    }
    if group == 0x0020 {
        match element {
            0x000D => {
                return Some(TagInfo { keyword: "StudyInstanceUID", vr: &[VRCode::UI], vm: "1" })
            }
            0x000E => {
                return Some(TagInfo { keyword: "SeriesInstanceUID", vr: &[VRCode::UI], vm: "1" })
            }
            0x0010 => return Some(TagInfo { keyword: "StudyID", vr: &[VRCode::SH], vm: "1" }),
            0x0011 => return Some(TagInfo { keyword: "SeriesNumber", vr: &[VRCode::IS], vm: "1" }),
            0x0012 => {
                return Some(TagInfo { keyword: "AcquisitionNumber", vr: &[VRCode::IS], vm: "1" })
            }
            0x0013 => {
                return Some(TagInfo { keyword: "InstanceNumber", vr: &[VRCode::IS], vm: "1" })
            }
            0x0020 => {
                return Some(TagInfo { keyword: "PatientOrientation", vr: &[VRCode::CS], vm: "2" })
            }
            0x0032 => {
                return Some(TagInfo {
                    keyword: "ImagePositionPatient",
                    vr: &[VRCode::DS],
                    vm: "3",
                })
            }
            0x0037 => {
                return Some(TagInfo {
                    keyword: "ImageOrientationPatient",
                    vr: &[VRCode::DS],
                    vm: "6",
                })
            }
            0x0052 => {
                return Some(TagInfo { keyword: "FrameOfReferenceUID", vr: &[VRCode::UI], vm: "1" })
            }
            0x0060 => return Some(TagInfo { keyword: "Laterality", vr: &[VRCode::CS], vm: "1" }),
            0x1002 => {
                return Some(TagInfo { keyword: "ImagesInAcquisition", vr: &[VRCode::IS], vm: "1" })
            }
            0x1040 => {
                return Some(TagInfo {
                    keyword: "PositionReferenceIndicator",
                    vr: &[VRCode::LO],
                    vm: "1",
                })
            }
            0x1041 => {
                return Some(TagInfo { keyword: "SliceLocation", vr: &[VRCode::DS], vm: "1" })
            }
            _ => {}
        }
    }
    if group == 0x0028 {
        match element {
            0x0002 => {
                return Some(TagInfo { keyword: "SamplesPerPixel", vr: &[VRCode::US], vm: "1" })
            }
            0x0004 => {
                return Some(TagInfo {
                    keyword: "PhotometricInterpretation",
                    vr: &[VRCode::CS],
                    vm: "1",
                })
            }
            0x0006 => {
                return Some(TagInfo { keyword: "PlanarConfiguration", vr: &[VRCode::US], vm: "1" })
            }
            0x0008 => {
                return Some(TagInfo { keyword: "NumberOfFrames", vr: &[VRCode::IS], vm: "1" })
            }
            0x0010 => return Some(TagInfo { keyword: "Rows", vr: &[VRCode::US], vm: "1" }),
            0x0011 => return Some(TagInfo { keyword: "Columns", vr: &[VRCode::US], vm: "1" }),
            0x0030 => return Some(TagInfo { keyword: "PixelSpacing", vr: &[VRCode::DS], vm: "2" }),
            0x0060 => {
                return Some(TagInfo { keyword: "CompressionCode", vr: &[VRCode::CS], vm: "1" })
            }
            0x0100 => {
                return Some(TagInfo { keyword: "BitsAllocated", vr: &[VRCode::US], vm: "1" })
            }
            0x0101 => return Some(TagInfo { keyword: "BitsStored", vr: &[VRCode::US], vm: "1" }),
            0x0102 => return Some(TagInfo { keyword: "HighBit", vr: &[VRCode::US], vm: "1" }),
            0x0103 => {
                return Some(TagInfo { keyword: "PixelRepresentation", vr: &[VRCode::US], vm: "1" })
            }
            0x0106 => {
                return Some(TagInfo {
                    keyword: "SmallestImagePixelValue",
                    vr: &[VRCode::US, VRCode::SS],
                    vm: "1",
                })
            }
            0x0107 => {
                return Some(TagInfo {
                    keyword: "LargestImagePixelValue",
                    vr: &[VRCode::US, VRCode::SS],
                    vm: "1",
                })
            }
            0x1050 => return Some(TagInfo { keyword: "WindowCenter", vr: &[VRCode::DS], vm: "1" }),
            0x1051 => return Some(TagInfo { keyword: "WindowWidth", vr: &[VRCode::DS], vm: "1" }),
            0x1052 => {
                return Some(TagInfo { keyword: "RescaleIntercept", vr: &[VRCode::DS], vm: "1" })
            }
            0x1053 => return Some(TagInfo { keyword: "RescaleSlope", vr: &[VRCode::DS], vm: "1" }),
            0x1054 => return Some(TagInfo { keyword: "RescaleType", vr: &[VRCode::LO], vm: "1" }),
            0x2110 => {
                return Some(TagInfo {
                    keyword: "LossyImageCompression",
                    vr: &[VRCode::CS],
                    vm: "1",
                })
            }
            _ => {}
        }
    }
    if group == 0x7FE0 {
        if element == 0x0010 {
            return Some(TagInfo { keyword: "PixelData", vr: &[VRCode::OW, VRCode::OB], vm: "1" });
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tag_module_is_compiled() {
        // This test should always pass if the module is compiled.
        assert!(true);
    }

    #[test]
    fn lookup_sop_class_uid() {
        let info = lookup_tag(0x0008, 0x0016).unwrap();
        assert_eq!(info.keyword, "SOPClassUID");
        assert_eq!(info.vr, &[VRCode::UI]);
    }

    #[test]
    fn lookup_pixel_data() {
        let info = lookup_tag(0x7FE0, 0x0010).unwrap();
        assert_eq!(info.keyword, "PixelData");
    }

    #[test]
    fn unknown_tag_returns_none() {
        assert!(lookup_tag(0xFFFF, 0x0000).is_none());
    }

    #[test]
    fn vr_from_bytes() {
        assert_eq!(VRCode::from_bytes(*b"US"), Some(VRCode::US));
        assert_eq!(VRCode::from_bytes(*b"UI"), Some(VRCode::UI));
        assert_eq!(VRCode::from_bytes(*b"XX"), None);
    }

    #[test]
    fn vr_round_trip_bytes() {
        for vr in [VRCode::AE, VRCode::US, VRCode::UI, VRCode::SQ, VRCode::OB] {
            assert_eq!(VRCode::from_bytes(vr.as_bytes()), Some(vr));
        }
    }
}
