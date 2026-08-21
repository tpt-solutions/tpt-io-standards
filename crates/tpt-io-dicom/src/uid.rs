//! UID registry — well-known DICOM UIDs for SOP classes and transfer
//! syntaxes, plus UID string validation (PS3.6 Annex A / PS3.5 §9).

use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

/// A well-known transfer syntax (PS3.5 §10 / PS3.6 Annex A).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TransferSyntax {
    /// `1.2.840.10008.1.2` — Implicit VR Little Endian.
    ImplicitVrLittleEndian,
    /// `1.2.840.10008.1.2.1` — Explicit VR Little Endian.
    ExplicitVrLittleEndian,
    /// `1.2.840.10008.1.2.1.99` — Deflated Explicit VR Little Endian.
    DeflatedExplicitVrLittleEndian,
    /// `1.2.840.10008.1.2.2` — Explicit VR Big Endian (retired but common).
    ExplicitVrBigEndian,
    /// `1.2.840.10008.1.2.4.50` — JPEG Baseline (Process 1).
    JpegBaseline,
    /// `1.2.840.10008.1.2.4.57` — JPEG Lossless, Non-Hierarchical.
    JpegLossless,
    /// `1.2.840.10008.1.2.4.70` — JPEG Lossless, First-Order Prediction.
    JpegLosslessFop,
    /// `1.2.840.10008.1.2.4.80` — JPEG-LS Lossless.
    JpegLsLossless,
    /// `1.2.840.10008.1.2.4.81` — JPEG-LS Lossy.
    JpegLsLossy,
    /// `1.2.840.10008.1.2.4.90` — JPEG 2000 Image Compression (Lossless Only).
    Jpeg2000Lossless,
    /// `1.2.840.10008.1.2.4.91` — JPEG 2000 Image Compression.
    Jpeg2000,
    /// `1.2.840.10008.1.2.5` — RLE Lossless.
    RleLossless,
}

impl TransferSyntax {
    /// The UID string for this transfer syntax.
    pub const fn uid(&self) -> &'static str {
        match self {
            TransferSyntax::ImplicitVrLittleEndian => "1.2.840.10008.1.2",
            TransferSyntax::ExplicitVrLittleEndian => "1.2.840.10008.1.2.1",
            TransferSyntax::DeflatedExplicitVrLittleEndian => "1.2.840.10008.1.2.1.99",
            TransferSyntax::ExplicitVrBigEndian => "1.2.840.10008.1.2.2",
            TransferSyntax::JpegBaseline => "1.2.840.10008.1.2.4.50",
            TransferSyntax::JpegLossless => "1.2.840.10008.1.2.4.57",
            TransferSyntax::JpegLosslessFop => "1.2.840.10008.1.2.4.70",
            TransferSyntax::JpegLsLossless => "1.2.840.10008.1.2.4.80",
            TransferSyntax::JpegLsLossy => "1.2.840.10008.1.2.4.81",
            TransferSyntax::Jpeg2000Lossless => "1.2.840.10008.1.2.4.90",
            TransferSyntax::Jpeg2000 => "1.2.840.10008.1.2.4.91",
            TransferSyntax::RleLossless => "1.2.840.10008.1.2.5",
        }
    }

    /// Resolve a transfer-syntax UID string to its enum value.
    pub fn from_uid(uid: &str) -> Option<TransferSyntax> {
        Some(match uid {
            "1.2.840.10008.1.2" => TransferSyntax::ImplicitVrLittleEndian,
            "1.2.840.10008.1.2.1" => TransferSyntax::ExplicitVrLittleEndian,
            "1.2.840.10008.1.2.1.99" => TransferSyntax::DeflatedExplicitVrLittleEndian,
            "1.2.840.10008.1.2.2" => TransferSyntax::ExplicitVrBigEndian,
            "1.2.840.10008.1.2.4.50" => TransferSyntax::JpegBaseline,
            "1.2.840.10008.1.2.4.57" => TransferSyntax::JpegLossless,
            "1.2.840.10008.1.2.4.70" => TransferSyntax::JpegLosslessFop,
            "1.2.840.10008.1.2.4.80" => TransferSyntax::JpegLsLossless,
            "1.2.840.10008.1.2.4.81" => TransferSyntax::JpegLsLossy,
            "1.2.840.10008.1.2.4.90" => TransferSyntax::Jpeg2000Lossless,
            "1.2.840.10008.1.2.4.91" => TransferSyntax::Jpeg2000,
            "1.2.840.10008.1.2.5" => TransferSyntax::RleLossless,
            _ => return None,
        })
    }

    /// Whether this transfer syntax encodes pixel data in compressed
    /// (encapsulated) form.
    pub const fn is_encapsulated_pixel_data(&self) -> bool {
        !matches!(
            self,
            TransferSyntax::ImplicitVrLittleEndian
                | TransferSyntax::ExplicitVrLittleEndian
                | TransferSyntax::DeflatedExplicitVrLittleEndian
                | TransferSyntax::ExplicitVrBigEndian
        )
    }

    /// Whether this crate can fully decode datasets using this transfer
    /// syntax. RLE counts because a pure-Rust decoder ships in this crate;
    /// the JPEG-family codecs require an external image backend.
    pub const fn is_fully_supported(&self) -> bool {
        matches!(
            self,
            TransferSyntax::ImplicitVrLittleEndian
                | TransferSyntax::ExplicitVrLittleEndian
                | TransferSyntax::ExplicitVrBigEndian
                | TransferSyntax::RleLossless
        )
    }
}

// PART2

/// A well-known SOP Class (PS3.6 Annex B, curated subset).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SopClass {
    /// Computed Radiography Image Storage.
    CrImageStorage,
    /// Computed Tomography Image Storage.
    CtImageStorage,
    /// MR Image Storage.
    MrImageStorage,
    /// Positron Emission Tomography Image Storage.
    PetImageStorage,
    /// Ultrasound Multi-frame Image Storage.
    UsMultiFrameImageStorage,
    /// Secondary Capture Image Storage.
    SecondaryCaptureImageStorage,
    /// Structured Report Storage (SR).
    SrStorage,
    /// Grayscale Softcopy Presentation State Storage.
    GrayscaleSoftcopyPresentationState,
    /// Encapsulated PDF Storage.
    EncapsulatedPdfStorage,
    /// Verification (echo) SOP Class.
    Verification,
}

impl SopClass {
    /// The UID string for this SOP class.
    pub const fn uid(&self) -> &'static str {
        match self {
            SopClass::CrImageStorage => "1.2.840.10008.5.1.4.1.1.1",
            SopClass::CtImageStorage => "1.2.840.10008.5.1.4.1.1.2",
            SopClass::MrImageStorage => "1.2.840.10008.5.1.4.1.1.4",
            SopClass::PetImageStorage => "1.2.840.10008.5.1.4.1.1.128",
            SopClass::UsMultiFrameImageStorage => "1.2.840.10008.5.1.4.1.1.3.1",
            SopClass::SecondaryCaptureImageStorage => "1.2.840.10008.5.1.4.1.1.7",
            SopClass::SrStorage => "1.2.840.10008.5.1.4.1.1.88.11",
            SopClass::GrayscaleSoftcopyPresentationState => "1.2.840.10008.5.1.4.1.1.11.1",
            SopClass::EncapsulatedPdfStorage => "1.2.840.10008.5.1.4.1.1.104.1",
            SopClass::Verification => "1.2.840.10008.1.1",
        }
    }

    /// Resolve a SOP Class UID string to its enum value.
    pub fn from_uid(uid: &str) -> Option<SopClass> {
        Some(match uid {
            "1.2.840.10008.5.1.4.1.1.1" => SopClass::CrImageStorage,
            "1.2.840.10008.5.1.4.1.1.2" => SopClass::CtImageStorage,
            "1.2.840.10008.5.1.4.1.1.4" => SopClass::MrImageStorage,
            "1.2.840.10008.5.1.4.1.1.128" => SopClass::PetImageStorage,
            "1.2.840.10008.5.1.4.1.1.3.1" => SopClass::UsMultiFrameImageStorage,
            "1.2.840.10008.5.1.4.1.1.7" => SopClass::SecondaryCaptureImageStorage,
            "1.2.840.10008.5.1.4.1.1.88.11" => SopClass::SrStorage,
            "1.2.840.10008.5.1.4.1.1.11.1" => SopClass::GrayscaleSoftcopyPresentationState,
            "1.2.840.10008.5.1.4.1.1.104.1" => SopClass::EncapsulatedPdfStorage,
            "1.2.840.10008.1.1" => SopClass::Verification,
            _ => return None,
        })
    }
}

/// Validate a UID string against the PS3.5 §9 rules: digits and dots only,
/// no leading/trailing dot, no empty components, no leading zeros in
/// components, and a 64-character total limit.
pub fn validate_uid(uid: &str) -> Result<(), String> {
    if uid.is_empty() {
        return Err("UID is empty".into());
    }
    if uid.len() > 64 {
        return Err(format!("UID exceeds 64 characters (len = {})", uid.len()).to_string());
    }
    if !uid.bytes().all(|b| b.is_ascii_digit() || b == b'.') {
        return Err("UID contains characters other than digits and dots".into());
    }
    if uid.starts_with('.') || uid.ends_with('.') {
        return Err("UID must not start or end with a dot".into());
    }
    if uid.contains("..") {
        return Err("UID contains an empty component".into());
    }
    for component in uid.split('.') {
        if component.len() > 1 && component.starts_with('0') {
            return Err(format!("UID component \"{component}\" has a leading zero"));
        }
    }
    Ok(())
}

/// Collect every known transfer syntax UID in the registry.
pub fn all_transfer_syntax_uids() -> Vec<&'static str> {
    use TransferSyntax::*;
    [
        ImplicitVrLittleEndian,
        ExplicitVrLittleEndian,
        DeflatedExplicitVrLittleEndian,
        ExplicitVrBigEndian,
        JpegBaseline,
        JpegLossless,
        JpegLosslessFop,
        JpegLsLossless,
        JpegLsLossy,
        Jpeg2000Lossless,
        Jpeg2000,
        RleLossless,
    ]
    .iter()
    .map(|ts| ts.uid())
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transfer_syntax_roundtrip() {
        for ts in [
            TransferSyntax::ImplicitVrLittleEndian,
            TransferSyntax::ExplicitVrLittleEndian,
            TransferSyntax::DeflatedExplicitVrLittleEndian,
            TransferSyntax::ExplicitVrBigEndian,
            TransferSyntax::RleLossless,
            TransferSyntax::Jpeg2000Lossless,
        ] {
            assert_eq!(TransferSyntax::from_uid(ts.uid()), Some(ts));
        }
        assert_eq!(TransferSyntax::from_uid("1.2.3.4"), None);
    }

    #[test]
    fn encapsulated_flags() {
        assert!(!TransferSyntax::ExplicitVrLittleEndian.is_encapsulated_pixel_data());
        assert!(TransferSyntax::RleLossless.is_encapsulated_pixel_data());
        assert!(TransferSyntax::RleLossless.is_fully_supported());
        assert!(!TransferSyntax::Jpeg2000.is_fully_supported());
    }

    #[test]
    fn sop_class_registry() {
        assert_eq!(SopClass::from_uid("1.2.840.10008.5.1.4.1.1.2"), Some(SopClass::CtImageStorage));
        assert_eq!(SopClass::from_uid("1.2.840.10008.1.1"), Some(SopClass::Verification));
        assert_eq!(SopClass::from_uid("1.2.3"), None);
    }

    #[test]
    fn uid_validation() {
        assert!(validate_uid("1.2.840.10008.1.2").is_ok());
        assert!(validate_uid("2.25.329800735698586629295641978511506172918").is_ok());
        assert!(validate_uid("1.2.03").is_err(), "leading zero");
        assert!(validate_uid(".1.2").is_err(), "leading dot");
        assert!(validate_uid("1.2.").is_err(), "trailing dot");
        assert!(validate_uid("1..2").is_err(), "empty component");
        assert!(validate_uid("1.2.a").is_err(), "non-digit");
        assert!(validate_uid(&"1".repeat(65)).is_err(), "too long");
    }

    #[test]
    fn registry_lists_all() {
        assert!(all_transfer_syntax_uids().len() >= 12);
    }
}
