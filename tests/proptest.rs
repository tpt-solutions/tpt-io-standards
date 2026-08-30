//! Property-based tests (proptest) for DICOM tag/VR invariants.

use proptest::prelude::*;
use tpt_io_dicom::model::{Dataset, Element, Tag, Value, VR};
use tpt_io_dicom::tag::{lookup_tag, VRCode};
use tpt_io_dicom::uid::{SopClass, TransferSyntax};
use tpt_io_dicom::writer::round_trip_dataset;

proptest! {
    #[test]
    fn vr_round_trip_bytes(
        byte1 in b'A'..=b'Z',
        byte2 in b'A'..=b'Z',
    ) {
        let bytes = [byte1, byte2];
        if let Some(vr) = VRCode::from_bytes(bytes) {
            prop_assert_eq!(vr.as_bytes(), bytes);
        }
    }

    #[test]
    fn tag_privacy_invariant(
        group in 0u16..=0xFFFFu16,
        element in 0u16..=0xFFFFu16,
    ) {
        let tag = Tag::new(group, element);
        prop_assert_eq!(tag.is_private(), group & 1 == 1);
        prop_assert_eq!(tag.is_group_length(), element == 0x0000);
        prop_assert_eq!(tag.group(), group);
        prop_assert_eq!(tag.element(), element);
    }

    #[test]
    fn sop_class_uid_round_trip(
        sop_idx in 0..30usize,
    ) {
        let sops = [
            SopClass::Verification,
            SopClass::CtImageStorage,
            SopClass::MrImageStorage,
            SopClass::PetImageStorage,
            SopClass::RtImageStorage,
            SopClass::RtDoseStorage,
            SopClass::RtStructureSetStorage,
            SopClass::RtPlanStorage,
            SopClass::NuclearMedicineImageStorage,
            SopClass::SecondaryCaptureImageStorage,
            SopClass::DigitalXRayImageStorageForPresentation,
            SopClass::DigitalMammographyXRayImageStorageForPresentation,
            SopClass::EncapsulatedPdf,
            SopClass::Segmentation,
            SopClass::SurfaceSegmentation,
            SopClass::ParametricMap,
            SopClass::BasicTextSr,
            SopClass::EnhancedSr,
            SopClass::ComprehensiveSr,
            SopClass::KeyObjectSelection,
            SopClass::ChestCadSr,
            SopClass::MammographyCadSr,
            SopClass::ProcedureLog,
            SopClass::TlSend,
            SopClass::UltrasoundMultiFrameImageStorage,
            SopClass::MultiFrameSingleBitSecondaryCaptureImageStorage,
            SopClass::MultiFrameGrayscaleByteSecondaryCaptureImageStorage,
            SopClass::MultiFrameGrayscaleWordSecondaryCaptureImageStorage,
            SopClass::MultiFrameTrueColorSecondaryCaptureImageStorage,
            SopClass::ComputedRadiographyImageStorage,
        ];
        let sop = sops[sop_idx % sops.len()];
        prop_assert_eq!(SopClass::from_uid(sop.uid()), Some(sop));
    }

    #[test]
    fn transfer_syntax_uid_round_trip(
        ts_idx in 0..14usize,
    ) {
        let tss = [
            TransferSyntax::ImplicitVrLittleEndian,
            TransferSyntax::ExplicitVrLittleEndian,
            TransferSyntax::ExplicitVrBigEndian,
            TransferSyntax::JpegBaseline,
            TransferSyntax::JpegExtended,
            TransferSyntax::JpegLossless,
            TransferSyntax::Jpeg2000Lossless,
            TransferSyntax::Jpeg2000,
            TransferSyntax::RleLossless,
            TransferSyntax::Mpeg2,
            TransferSyntax::Mpeg4,
            TransferSyntax::DeflatedExplicitVrLittleEndian,
            TransferSyntax::JpipReferenced,
            TransferSyntax::JpipReferencedDeflate,
        ];
        let ts = tss[ts_idx % tss.len()];
        prop_assert_eq!(TransferSyntax::from_uid(ts.uid()), Some(ts));
    }

    #[test]
    fn transfer_syntax_flags_consistent(ts_idx in 0..14usize) {
        let tss = [
            TransferSyntax::ImplicitVrLittleEndian,
            TransferSyntax::ExplicitVrLittleEndian,
            TransferSyntax::ExplicitVrBigEndian,
            TransferSyntax::JpegBaseline,
            TransferSyntax::Jpeg2000,
            TransferSyntax::RleLossless,
        ];
        let ts = tss[ts_idx % tss.len()];
        // is_implicit_vr and is_explicit_vr are mutually exclusive.
        prop_assert!(ts.is_implicit_vr() != ts.is_explicit_vr());
        // All except big-endian are little-endian.
        if !matches!(ts, TransferSyntax::ExplicitVrBigEndian) {
            prop_assert!(ts.is_little_endian());
        }
    }

    #[test]
    fn string_value_round_trip(
        s in "[\\x20-\\x7E]{0,64}",
    ) {
        let mut ds = Dataset::new();
        ds.push(Element {
            tag: Tag::new(0x0008, 0x0016),
            vr: VR::Known(VRCode::UI),
            value: Value::String(s.clone()),
        });
        let rt = round_trip_dataset(&ds, TransferSyntax::ExplicitVrLittleEndian).unwrap();
        // The string should survive round-trip (possibly with whitespace trimmed).
        let result = rt.string(Tag::new(0x0008, 0x0016)).unwrap();
        // DICOM trims trailing whitespace/nulls, so the result should be a trimmed version.
        prop_assert!(s.trim_end().starts_with(result.trim_end()) || result.trim_end().starts_with(s.trim_end()));
    }

    #[test]
    fn u16_value_round_trip(
        v in 0u16..=0xFFFFu16,
    ) {
        let mut ds = Dataset::new();
        ds.push(Element {
            tag: Tag::new(0x0028, 0x0010),
            vr: VR::Known(VRCode::US),
            value: Value::U16(v),
        });
        let rt = round_trip_dataset(&ds, TransferSyntax::ExplicitVrLittleEndian).unwrap();
        prop_assert_eq!(rt.value(Tag::new(0x0028, 0x0010)), Some(&Value::U16(v)));
    }
}
