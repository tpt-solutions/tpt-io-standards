//! Anonymization utilities implementing a practical subset of the DICOM
//! Basic Application Level Confidentiality Profile (PS3.15 Annex E):
//! remove direct patient identifiers, blank date/time fields, and strip
//! private (odd-group) elements.

use alloc::collections::BTreeMap;
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

use crate::dataset::Dataset;
use crate::element::DataElement;
use crate::tag::Tag;
use crate::vr::Vr;

/// Tags removed outright by [`anonymize`] (direct identifiers, PS3.15
/// Table E.1-1 subset).
pub const REMOVED_TAGS: &[Tag] = &[
    Tag::new(0x0010, 0x0010), // PatientName
    Tag::new(0x0010, 0x0020), // PatientID
    Tag::new(0x0010, 0x1000), // OtherPatientIDs
    Tag::new(0x0010, 0x2154), // PatientPhoneNumbers
    Tag::new(0x0010, 0x4000), // PatientComments
    Tag::new(0x0008, 0x0090), // ReferringPhysicianName
    Tag::new(0x0008, 0x1060), // NameOfPhysiciansReadingStudy
    Tag::new(0x0008, 0x1070), // OperatorsName
    Tag::new(0x0008, 0x0080), // InstitutionName
    Tag::new(0x0008, 0x0081), // InstitutionAddress
];

/// Tags replaced with a fixed dummy value by [`anonymize`].
pub const REPLACED_TAGS: &[Tag] = &[
    Tag::new(0x0010, 0x0030), // PatientBirthDate → 19000101
    Tag::new(0x0008, 0x0050), // AccessionNumber → "" (zero length)
];

const DUMMY_BIRTH_DATE: &str = "19000101";

/// Options controlling anonymization behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AnonymizeOptions {
    /// Remove all private (odd-group) elements. Recommended: yes.
    pub remove_private_tags: bool,
    /// Remove study/series/instance dates, times, and patient age.
    pub blank_dates: bool,
}

impl Default for AnonymizeOptions {
    fn default() -> Self {
        Self { remove_private_tags: true, blank_dates: true }
    }
}

const DATE_TIME_TAGS: &[Tag] = &[
    Tag::new(0x0008, 0x0020), // StudyDate
    Tag::new(0x0008, 0x0021), // SeriesDate
    Tag::new(0x0008, 0x0022), // AcquisitionDate
    Tag::new(0x0008, 0x0023), // ContentDate
    Tag::new(0x0008, 0x0030), // StudyTime
    Tag::new(0x0008, 0x0031), // SeriesTime
    Tag::new(0x0008, 0x0032), // AcquisitionTime
    Tag::new(0x0008, 0x0033), // ContentTime
    Tag::new(0x0010, 0x1010), // PatientAge
];

/// Apply the confidentiality profile to a clone of `ds` and return it.
///
/// UIDs are *not* remapped here — use [`UidRemapper`] separately if
/// de-identified studies must be unlinkable to their originals.
pub fn anonymize(ds: &Dataset, options: AnonymizeOptions) -> Dataset {
    let mut out = ds.clone();
    for tag in REMOVED_TAGS {
        out.remove(*tag);
    }
    for tag in REPLACED_TAGS {
        let vr = crate::dict::vr_of(*tag).unwrap_or(Vr::Lo);
        let value: Vec<u8> =
            if tag.element == 0x0030 { DUMMY_BIRTH_DATE.as_bytes().to_vec() } else { Vec::new() };
        out.upsert(DataElement::new(*tag, vr, value));
    }
    if options.blank_dates {
        for tag in DATE_TIME_TAGS {
            out.remove(*tag);
        }
    }
    if options.remove_private_tags {
        out.elements.retain(|e| !e.tag.is_private());
    }
    out
}

/// Stable UID remapping table shared across a series so that de-identified
/// studies stay internally consistent but are unlinkable to their sources.
pub struct UidRemapper {
    map: BTreeMap<String, String>,
    generated: BTreeMap<String, ()>,
    prefix: String,
    next: u64,
}

impl UidRemapper {
    /// New remapper generating UIDs under `prefix` (use your organization
    /// root per PS3.5 §9 UID rules).
    pub fn new(prefix: impl Into<String>) -> Self {
        Self { map: BTreeMap::new(), generated: BTreeMap::new(), prefix: prefix.into(), next: 1 }
    }

    /// Map `uid` to a stable replacement, creating one on first use.
    ///
    /// Already-generated replacement values are returned unchanged so that
    /// re-rewriting a dataset is idempotent.
    pub fn map(&mut self, uid: &str) -> String {
        if let Some(existing) = self.map.get(uid) {
            return existing.clone();
        }
        if self.generated.contains_key(uid) {
            return uid.to_string(); // already a fresh UID: leave as-is
        }
        let fresh = format!("{}.{}.{}", self.prefix, self.next, 1000 + self.next % 7);
        self.next += 1;
        self.map.insert(uid.to_string(), fresh.clone());
        self.generated.insert(fresh.clone(), ());
        fresh
    }

    /// Number of remapped UIDs so far.
    pub fn len(&self) -> usize {
        self.map.len()
    }

    /// Whether nothing has been remapped yet.
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    /// Rewrite every non-empty UI element of `ds` through this mapper.
    pub fn rewrite(&mut self, ds: &mut Dataset) {
        for e in &mut ds.elements {
            if e.vr == Vr::Ui {
                if let Some(s) = e.as_string() {
                    let trimmed = s.trim().to_string();
                    if !trimmed.is_empty() && crate::uid::validate_uid(&trimmed).is_ok() {
                        let mapped = self.map(&trimmed);
                        *e = DataElement::text(e.tag, Vr::Ui, &mapped);
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dataset::Encoding;

    fn sample() -> Dataset {
        let mut ds = Dataset {
            elements: Vec::new(),
            encoding: Encoding { big_endian: false, explicit_vr: true },
        };
        ds.upsert(DataElement::text(Tag::new(0x0010, 0x0010), Vr::Pn, "Doe^John"));
        ds.upsert(DataElement::text(Tag::new(0x0010, 0x0020), Vr::Lo, "PID-1"));
        ds.upsert(DataElement::text(Tag::new(0x0010, 0x0030), Vr::Da, "19750601"));
        ds.upsert(DataElement::text(Tag::new(0x0008, 0x0020), Vr::Da, "20240101"));
        ds.upsert(DataElement::text(Tag::new(0x0008, 0x0050), Vr::Sh, "ACC-1"));
        ds.upsert(DataElement::new(Tag::new(0x0009, 0x0001), Vr::Lo, b"private".to_vec()));
        ds.upsert(DataElement::text(Tag::new(0x0008, 0x0018), Vr::Ui, "1.2.3.4"));
        ds
    }

    #[test]
    fn identifiers_removed_and_dates_replaced() {
        let out = anonymize(&sample(), AnonymizeOptions::default());
        assert!(out.element(Tag::new(0x0010, 0x0010)).is_none(), "name removed");
        assert!(out.element(Tag::new(0x0010, 0x0020)).is_none(), "id removed");
        assert_eq!(out.string(Tag::new(0x0010, 0x0030)).as_deref(), Some("19000101"));
        assert_eq!(out.string(Tag::new(0x0008, 0x0050)).as_deref(), Some(""));
        assert!(out.element(Tag::new(0x0008, 0x0020)).is_none(), "study date blanked");
        assert!(out.element(Tag::new(0x0009, 0x0001)).is_none(), "private removed");
        assert!(out.element(Tag::new(0x0008, 0x0018)).is_some(), "SOP UID kept");
    }

    #[test]
    fn keep_private_option() {
        let out = anonymize(
            &sample(),
            AnonymizeOptions { remove_private_tags: false, blank_dates: false },
        );
        assert!(out.element(Tag::new(0x0009, 0x0001)).is_some());
        assert!(out.element(Tag::new(0x0008, 0x0020)).is_some());
    }

    #[test]
    fn uid_remapping_is_stable_and_valid() {
        let mut rm = UidRemapper::new("2.25");
        let mut ds = sample();
        rm.rewrite(&mut ds);
        let sop = ds.string(Tag::new(0x0008, 0x0018)).unwrap();
        assert_ne!(sop.trim(), "1.2.3.4");
        assert!(crate::uid::validate_uid(sop.trim()).is_ok());
        rm.rewrite(&mut ds); // stable
        assert_eq!(ds.string(Tag::new(0x0008, 0x0018)).unwrap().trim(), sop.trim());
        assert!(!rm.is_empty());
    }
}
