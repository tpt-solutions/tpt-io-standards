//! Tag dictionary — a curated subset of standard DICOM data elements
//! (PS3.6) used for Implicit VR inference, name lookup, and validation.
//!
//! Stored as a tag-sorted `static` table searched by binary search so the
//! dictionary works in `no_std` builds with zero runtime initialization.

use crate::tag::Tag;
use crate::vr::Vr;

/// A standard data element entry from the dictionary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TagInfo {
    /// The tag.
    pub tag: Tag,
    /// VR mandated by PS3.6.
    pub vr: Vr,
    /// Value Multiplicity: `None` means "1 or more" (`1-n`).
    pub vm: Option<u32>,
    /// Human-readable keyword / name.
    pub name: &'static str,
}

const fn info(group: u16, element: u16, vr: Vr, vm: Option<u32>, name: &'static str) -> TagInfo {
    TagInfo { tag: Tag::new(group, element), vr, vm, name }
}

/// The standard tag dictionary (curated subset covering the File Meta,
/// SOP Common, Patient, General Study/Series/Equipment, Image Plane,
/// Image Pixel, and Pixel Data modules).
pub static TAG_DICTIONARY: &[TagInfo] = &[
    // --- File Meta Information (PS3.10 §7.1) ---
    info(0x0002, 0x0000, Vr::Ul, Some(1), "FileMetaInformationGroupLength"),
    info(0x0002, 0x0001, Vr::Ob, Some(1), "FileMetaInformationVersion"),
    info(0x0002, 0x0002, Vr::Ui, Some(1), "MediaStorageSOPClassUID"),
    info(0x0002, 0x0003, Vr::Ui, Some(1), "MediaStorageSOPInstanceUID"),
    info(0x0002, 0x0010, Vr::Ui, Some(1), "TransferSyntaxUID"),
    info(0x0002, 0x0012, Vr::Ui, Some(1), "ImplementationClassUID"),
    info(0x0002, 0x0013, Vr::Sh, Some(1), "ImplementationVersionName"),
    info(0x0002, 0x0016, Vr::Ae, Some(1), "SourceApplicationEntityTitle"),
    // --- SOP Common (PS3.3 C.12) ---
    info(0x0008, 0x0005, Vr::Cs, Some(1), "SpecificCharacterSet"),
    info(0x0008, 0x0008, Vr::Cs, None, "ImageType"),
    info(0x0008, 0x0012, Vr::Da, Some(1), "InstanceCreationDate"),
    info(0x0008, 0x0013, Vr::Tm, Some(1), "InstanceCreationTime"),
    info(0x0008, 0x0016, Vr::Ui, Some(1), "SOPClassUID"),
    info(0x0008, 0x0018, Vr::Ui, Some(1), "SOPInstanceUID"),
    info(0x0008, 0x0020, Vr::Da, Some(1), "StudyDate"),
    info(0x0008, 0x0021, Vr::Da, Some(1), "SeriesDate"),
    info(0x0008, 0x0022, Vr::Da, Some(1), "AcquisitionDate"),
    info(0x0008, 0x0023, Vr::Da, Some(1), "ContentDate"),
    info(0x0008, 0x0030, Vr::Tm, Some(1), "StudyTime"),
    info(0x0008, 0x0031, Vr::Tm, Some(1), "SeriesTime"),
    info(0x0008, 0x0032, Vr::Tm, Some(1), "AcquisitionTime"),
    info(0x0008, 0x0033, Vr::Tm, Some(1), "ContentTime"),
    info(0x0008, 0x0050, Vr::Sh, Some(1), "AccessionNumber"),
    info(0x0008, 0x0060, Vr::Cs, Some(1), "Modality"),
    info(0x0008, 0x0070, Vr::Lo, Some(1), "Manufacturer"),
    info(0x0008, 0x0080, Vr::Lo, Some(1), "InstitutionName"),
    info(0x0008, 0x0090, Vr::Pn, Some(1), "ReferringPhysicianName"),
    info(0x0008, 0x1030, Vr::Lo, Some(1), "StudyDescription"),
    info(0x0008, 0x103E, Vr::Lo, Some(1), "SeriesDescription"),
    // --- Patient (PS3.3 C.7) ---
    info(0x0010, 0x0010, Vr::Pn, Some(1), "PatientName"),
    info(0x0010, 0x0020, Vr::Lo, Some(1), "PatientID"),
    info(0x0010, 0x0030, Vr::Da, Some(1), "PatientBirthDate"),
    info(0x0010, 0x0040, Vr::Cs, Some(1), "PatientSex"),
    info(0x0010, 0x1000, Vr::Lo, None, "OtherPatientIDs"),
    info(0x0010, 0x1010, Vr::As, Some(1), "PatientAge"),
    info(0x0010, 0x1020, Vr::Ds, Some(1), "PatientSize"),
    info(0x0010, 0x1030, Vr::Ds, Some(1), "PatientWeight"),
    // --- General Equipment (PS3.3 C.7.5) ---
    info(0x0018, 0x0015, Vr::Cs, Some(1), "BodyPartExamined"),
    info(0x0018, 0x0050, Vr::Ds, Some(1), "SliceThickness"),
    info(0x0018, 0x0060, Vr::Ds, Some(1), "KVP"),
    // --- General Study (PS3.3 C.7.2) ---
    info(0x0020, 0x000D, Vr::Ui, Some(1), "StudyInstanceUID"),
    // --- General Series (PS3.3 C.7.3) ---
    info(0x0020, 0x000E, Vr::Ui, Some(1), "SeriesInstanceUID"),
    info(0x0020, 0x0010, Vr::Sh, Some(1), "StudyID"),
    info(0x0020, 0x0011, Vr::Is, Some(1), "SeriesNumber"),
    info(0x0020, 0x0012, Vr::Is, Some(1), "AcquisitionNumber"),
    info(0x0020, 0x0013, Vr::Is, Some(1), "InstanceNumber"),
    // --- Image Plane (PS3.3 C.7.6.2) ---
    info(0x0020, 0x0032, Vr::Ds, Some(3), "ImagePositionPatient"),
    info(0x0020, 0x0037, Vr::Ds, Some(6), "ImageOrientationPatient"),
    info(0x0020, 0x0052, Vr::Ui, Some(1), "FrameOfReferenceUID"),
    // --- Image Pixel (PS3.3 C.7.6.3) ---
    info(0x0028, 0x0002, Vr::Us, Some(1), "SamplesPerPixel"),
    info(0x0028, 0x0004, Vr::Cs, Some(1), "PhotometricInterpretation"),
    info(0x0028, 0x0008, Vr::Is, Some(1), "NumberOfFrames"),
    info(0x0028, 0x0010, Vr::Us, Some(1), "Rows"),
    info(0x0028, 0x0011, Vr::Us, Some(1), "Columns"),
    info(0x0028, 0x0030, Vr::Ds, Some(2), "PixelSpacing"),
    info(0x0028, 0x0100, Vr::Us, Some(1), "BitsAllocated"),
    info(0x0028, 0x0101, Vr::Us, Some(1), "BitsStored"),
    info(0x0028, 0x0102, Vr::Us, Some(1), "HighBit"),
    info(0x0028, 0x0103, Vr::Us, Some(1), "PixelRepresentation"),
    // --- Pixel Data ---
    info(0x7FE0, 0x0010, Vr::Ob, Some(1), "PixelData"),
];

/// Look up a standard tag in the dictionary.
pub fn lookup(tag: Tag) -> Option<&'static TagInfo> {
    TAG_DICTIONARY.binary_search_by(|probe| probe.tag.cmp(&tag)).ok().map(|i| &TAG_DICTIONARY[i])
}

/// The VR mandated by PS3.6 for `tag`, if it is in the dictionary.
pub fn vr_of(tag: Tag) -> Option<Vr> {
    lookup(tag).map(|i| i.vr)
}

/// The standard keyword/name of `tag`, if known.
pub fn name_of(tag: Tag) -> Option<&'static str> {
    lookup(tag).map(|i| i.name)
}

/// All dictionary entries, ordered by tag.
pub fn all_entries() -> impl Iterator<Item = &'static TagInfo> {
    TAG_DICTIONARY.iter()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dictionary_is_sorted() {
        let mut prev = Tag::new(0, 0);
        for e in TAG_DICTIONARY {
            assert!(e.tag > prev, "dictionary out of order at {e:?}");
            prev = e.tag;
        }
    }

    #[test]
    fn lookup_common_tags() {
        assert_eq!(name_of(Tag::new(0x0010, 0x0010)), Some("PatientName"));
        assert_eq!(vr_of(Tag::new(0x7FE0, 0x0010)), Some(Vr::Ob));
        assert_eq!(vr_of(Tag::new(0x0028, 0x0010)), Some(Vr::Us));
        assert_eq!(lookup(Tag::new(0x1234, 0x5678)), None);
    }
}
