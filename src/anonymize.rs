//! DICOM anonymization utilities (PS3.15 Annex E — Basic Application Level Confidentiality Profile).

use alloc::string::String;
use alloc::vec::Vec;

use crate::model::{Dataset, Element, Tag, Value, VR};
use crate::tag::VRCode;

/// Anonymization action for a tag.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnonAction {
    /// Remove the tag entirely.
    Remove,
    /// Replace with a dummy value (zero-length or default).
    Empty,
    /// Replace with a consistent pseudonym (hashed from original).
    Pseudonymize,
    /// Keep the tag as-is.
    Keep,
}

/// Anonymization profile: maps tags to actions.
#[derive(Debug, Clone, Default)]
pub struct AnonymizationProfile {
    /// Tag-specific overrides.
    overrides: Vec<(Tag, AnonAction)>,
    /// Default action for patient-identifying tags.
    default_patient_action: AnonAction,
    /// Default action for general tags.
    default_action: AnonAction,
}

impl AnonymizationProfile {
    /// Create a basic profile that removes all patient-identifying information.
    pub fn basic() -> Self {
        Self {
            overrides: Vec::new(),
            default_patient_action: AnonAction::Remove,
            default_action: AnonAction::Keep,
        }
    }

    /// Create a profile that pseudonymizes patient data (replaces with consistent hashes).
    pub fn pseudonymize() -> Self {
        Self {
            overrides: Vec::new(),
            default_patient_action: AnonAction::Pseudonymize,
            default_action: AnonAction::Keep,
        }
    }

    /// Add a tag-specific override.
    pub fn with_override(mut self, tag: Tag, action: AnonAction) -> Self {
        self.overrides.push((tag, action));
        self
    }

    /// Get the action for a given tag.
    pub fn action_for(&self, tag: Tag) -> AnonAction {
        // Check overrides first.
        for (t, action) in &self.overrides {
            if *t == tag {
                return *action;
            }
        }
        // Check if this is a patient-identifying tag.
        if is_patient_identifying(tag) {
            self.default_patient_action
        } else {
            self.default_action
        }
    }
}

/// Tags that contain patient-identifying information (subset of PS3.15 Annex E).
fn is_patient_identifying(tag: Tag) -> bool {
    matches!(
        (tag.group(), tag.element()),
        (0x0008, 0x0014) | // Instance Creator UID
        (0x0008, 0x0018) | // SOP Instance UID
        (0x0008, 0x0050) | // Accession Number
        (0x0008, 0x0080) | // Institution Name
        (0x0008, 0x0081) | // Institution Address
        (0x0008, 0x0090) | // Referring Physician's Name
        (0x0008, 0x0092) | // Referring Physician's Address
        (0x0008, 0x0094) | // Referring Physician's Telephone
        (0x0008, 0x1010) | // Station Name
        (0x0008, 0x1030) | // Study Description
        (0x0008, 0x103E) | // Series Description
        (0x0008, 0x1048) | // Physician(s) of Record
        (0x0008, 0x1060) | // Name of Physician(s) Reading Study
        (0x0010, 0x0010) | // Patient's Name
        (0x0010, 0x0020) | // Patient ID
        (0x0010, 0x0021) | // Issuer of Patient ID
        (0x0010, 0x0030) | // Patient's Birth Date
        (0x0010, 0x0032) | // Patient's Birth Time
        (0x0010, 0x0040) | // Patient's Sex
        (0x0010, 0x1000) | // Other Patient IDs
        (0x0010, 0x1001) | // Other Patient Names
        (0x0010, 0x1010) | // Patient's Age
        (0x0010, 0x1020) | // Patient's Size
        (0x0010, 0x1030) | // Patient's Weight
        (0x0010, 0x2154) | // Patient's Telephone Numbers
        (0x0010, 0x2160) | // Ethnic Group
        (0x0010, 0x21B0) | // Additional Patient History
        (0x0010, 0x21F0) | // Patient's Religious Preference
        (0x0010, 0x4000) | // Patient Comments
        (0x0012, 0x0010) | // Clinical Trial Sponsor Name
        (0x0012, 0x0020) | // Clinical Trial Protocol ID
        (0x0012, 0x0021) | // Clinical Trial Protocol Name
        (0x0012, 0x0030) | // Clinical Trial Site ID
        (0x0012, 0x0031) | // Clinical Trial Site Name
        (0x0012, 0x0040) | // Clinical Trial Subject ID
        (0x0012, 0x0042) | // Clinical Trial Subject Reading ID
        (0x0038, 0x0010) | // Admission ID
        (0x0038, 0x0020) | // Admitting Date
        (0x0038, 0x0021) | // Admitting Time
        (0x0038, 0x0500) | // Patient State
        (0x0040, 0x0006) | // Scheduled Performing Physician's Name
        (0x0040, 0x000B) | // Scheduled Performing Physician ID Sequence
        (0x0040, 0x2016) | // Placer Order Number / Imaging Service Request
        (0x0040, 0x2017) | // Filler Order Number / Imaging Service Request
        (0x0040, 0xA730) | // Content Sequence (may contain patient info)
        (0x0088, 0x0140) | // Storage Media File-set UID
        (0x3006, 0x0024) | // Referenced Frame of Reference UID
        (0x3006, 0x00C2) | // Related Frame of Reference UID
        (0x0008, 0x0018)   // SOP Instance UID
    )
}

/// Anonymize a dataset according to the given profile.
pub fn anonymize_dataset(dataset: &mut Dataset, profile: &AnonymizationProfile) {
    let mut new_elements = Vec::new();
    for element in &dataset.elements {
        match profile.action_for(element.tag) {
            AnonAction::Remove => {
                // Skip this element.
            }
            AnonAction::Empty => {
                let mut elem = element.clone();
                elem.value = Value::String(String::new());
                new_elements.push(elem);
            }
            AnonAction::Pseudonymize => {
                let mut elem = element.clone();
                elem.value = pseudonymize_value(&element.value);
                new_elements.push(elem);
            }
            AnonAction::Keep => {
                new_elements.push(element.clone());
            }
        }
    }
    dataset.elements = new_elements;
}

/// Pseudonymize a value by hashing it.
fn pseudonymize_value(value: &Value) -> Value {
    match value {
        Value::String(s) => {
            // Simple hash-based pseudonymization.
            let hash = simple_hash(s.as_bytes());
            Value::String(alloc::format!("ANON-{hash:016X}"))
        }
        Value::Raw(bytes) => {
            let hash = simple_hash(bytes);
            Value::String(alloc::format!("ANON-{hash:016X}"))
        }
        other => other.clone(),
    }
}

/// Simple FNV-1a hash for pseudonymization.
fn simple_hash(data: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for &byte in data {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_anonymization_removes_patient_name() {
        let mut ds = Dataset::new();
        ds.push(Element {
            tag: Tag::new(0x0010, 0x0010),
            vr: VR::Known(VRCode::PN),
            value: Value::String("Doe^John".into()),
        });
        ds.push(Element {
            tag: Tag::new(0x0008, 0x0016),
            vr: VR::Known(VRCode::UI),
            value: Value::String("1.2.840.10008.5.1.4.1.1.2".into()),
        });

        let profile = AnonymizationProfile::basic();
        anonymize_dataset(&mut ds, &profile);

        // Patient name should be removed.
        assert!(ds.get(Tag::new(0x0010, 0x0010)).is_none());
        // SOP Class UID should be kept.
        assert!(ds.get(Tag::new(0x0008, 0x0016)).is_some());
    }

    #[test]
    fn pseudonymize_replaces_with_hash() {
        let mut ds = Dataset::new();
        ds.push(Element {
            tag: Tag::new(0x0010, 0x0010),
            vr: VR::Known(VRCode::PN),
            value: Value::String("Doe^John".into()),
        });

        let profile = AnonymizationProfile::pseudonymize();
        anonymize_dataset(&mut ds, &profile);

        let elem = ds.get(Tag::new(0x0010, 0x0010)).unwrap();
        match &elem.value {
            Value::String(s) => {
                assert!(s.starts_with("ANON-"));
            }
            _ => panic!("expected string value"),
        }
    }

    #[test]
    fn override_action() {
        let mut ds = Dataset::new();
        ds.push(Element {
            tag: Tag::new(0x0010, 0x0010),
            vr: VR::Known(VRCode::PN),
            value: Value::String("Doe^John".into()),
        });

        let profile = AnonymizationProfile::basic()
            .with_override(Tag::new(0x0010, 0x0010), AnonAction::Keep);
        anonymize_dataset(&mut ds, &profile);

        // Patient name should be kept due to override.
        assert!(ds.get(Tag::new(0x0010, 0x0010)).is_some());
    }
}
