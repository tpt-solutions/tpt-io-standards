//! Study / Series / Instance hierarchy navigation (PS3.3 information
//! model): group parsed datasets into a Patient → Study → Series → Instance
//! tree keyed by their UID elements.

use alloc::collections::BTreeMap;
use alloc::string::String;

use crate::dataset::Dataset;
use crate::tag::Tag;

const PATIENT_ID: Tag = Tag::new(0x0010, 0x0020);
const STUDY_UID: Tag = Tag::new(0x0020, 0x000D);
const SERIES_UID: Tag = Tag::new(0x0020, 0x000E);
const SOP_UID: Tag = Tag::new(0x0008, 0x0018);

/// One instance (image) in a series: its SOP Instance UID plus dataset.
#[derive(Debug, Clone)]
pub struct Instance<'a> {
    /// SOP Instance UID `(0008,0018)`, if present.
    pub sop_instance_uid: Option<String>,
    /// The source dataset.
    pub dataset: &'a Dataset,
}

/// One series within a study.
#[derive(Debug, Default)]
pub struct Series<'a> {
    /// Series Instance UID.
    pub uid: String,
    /// Instances ordered by SOP Instance UID for deterministic traversal.
    pub instances: BTreeMap<String, Instance<'a>>,
}

/// One study for a patient.
#[derive(Debug, Default)]
pub struct Study<'a> {
    /// Study Instance UID.
    pub uid: String,
    /// Series keyed by Series Instance UID.
    pub series: BTreeMap<String, Series<'a>>,
}

/// One patient with their studies.
#[derive(Debug, Default)]
pub struct Patient<'a> {
    /// Patient ID `(0010,0020)` (empty string when absent).
    pub id: String,
    /// Studies keyed by Study Instance UID.
    pub studies: BTreeMap<String, Study<'a>>,
}

/// The full hierarchy built from a set of datasets (borrowed, zero-copy).
#[derive(Debug, Default)]
pub struct Hierarchy<'a> {
    patients: BTreeMap<String, Patient<'a>>,
}

impl<'a> Hierarchy<'a> {
    /// Build a hierarchy from datasets (e.g. all files of a CT volume).
    pub fn build(datasets: &'a [Dataset]) -> Self {
        let mut h = Hierarchy { patients: BTreeMap::new() };
        for ds in datasets {
            h.insert(ds);
        }
        h
    }

    /// Insert one dataset into the hierarchy.
    pub fn insert(&mut self, ds: &'a Dataset) {
        let patient_id = ds.string(PATIENT_ID).unwrap_or_default();
        let patient = self
            .patients
            .entry(patient_id.clone())
            .or_insert_with(|| Patient { id: patient_id, studies: BTreeMap::new() });
        let study_uid = ds.string(STUDY_UID).unwrap_or_else(|| "unknown-study".into());
        let study = patient
            .studies
            .entry(study_uid.clone())
            .or_insert_with(|| Study { uid: study_uid, series: BTreeMap::new() });
        let series_uid = ds.string(SERIES_UID).unwrap_or_else(|| "unknown-series".into());
        let series = study
            .series
            .entry(series_uid.clone())
            .or_insert_with(|| Series { uid: series_uid, instances: BTreeMap::new() });
        let sop = ds.string(SOP_UID).unwrap_or_default();
        series.instances.insert(
            sop.clone(),
            Instance {
                sop_instance_uid: if sop.is_empty() { None } else { Some(sop) },
                dataset: ds,
            },
        );
    }

    /// All patients, ordered by ID.
    pub fn patients(&self) -> impl Iterator<Item = &Patient<'a>> {
        self.patients.values()
    }

    /// Look up a patient by ID.
    pub fn patient(&self, id: &str) -> Option<&Patient<'a>> {
        self.patients.get(id)
    }

    /// Navigate directly to an instance by UIDs.
    #[allow(clippy::type_complexity)]
    pub fn find(
        &self,
        patient_id: &str,
        study_uid: &str,
        series_uid: &str,
        sop_uid: &str,
    ) -> Option<&Instance<'a>> {
        self.patient(patient_id)?
            .studies
            .get(study_uid)?
            .series
            .get(series_uid)?
            .instances
            .get(sop_uid)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dataset::{Dataset, Encoding};
    use crate::element::DataElement;
    use crate::vr::Vr;

    fn make(patient: &str, study: &str, series: &str, sop: &str) -> Dataset {
        let mut ds = Dataset {
            elements: Vec::new(),
            encoding: Encoding { big_endian: false, explicit_vr: true },
        };
        ds.upsert(DataElement::text(PATIENT_ID, Vr::Lo, patient));
        ds.upsert(DataElement::text(STUDY_UID, Vr::Ui, study));
        ds.upsert(DataElement::text(SERIES_UID, Vr::Ui, series));
        ds.upsert(DataElement::text(SOP_UID, Vr::Ui, sop));
        ds
    }

    #[test]
    fn groups_by_uids() {
        let dss = vec![
            make("P1", "S", "SE1", "I1"),
            make("P1", "S", "SE1", "I2"),
            make("P1", "S", "SE2", "I3"),
            make("P2", "T", "SE3", "I4"),
        ];
        let refs: Vec<Dataset> = dss;
        let h = Hierarchy::build(&refs);
        assert_eq!(h.patients().count(), 2);
        assert_eq!(h.patient("P1").unwrap().studies["S"].series.len(), 2);
        let found = h.find("P1", "S", "SE1", "I2").unwrap();
        assert_eq!(found.sop_instance_uid.as_deref(), Some("I2"));
        assert!(h.find("P1", "S", "SE1", "NOPE").is_none());
    }

    #[test]
    fn missing_uids_get_placeholders() {
        let mut ds = Dataset::default();
        ds.upsert(DataElement::text(Tag::new(0x0008, 0x0060), Vr::Cs, "CT"));
        let refs = vec![ds];
        let h = Hierarchy::build(&refs);
        // Lands under the "" patient in placeholder buckets.
        let p = h.patient("").unwrap();
        assert!(p.studies.contains_key("unknown-study"));
    }
}
