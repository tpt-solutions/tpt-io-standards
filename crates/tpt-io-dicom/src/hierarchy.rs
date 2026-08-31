//! Study/series/instance hierarchy navigation (PS3.3 §7).

use alloc::string::String;
use alloc::vec::Vec;

use crate::model::{Dataset, Tag, Value};

/// A DICOM Study: a collection of series.
#[derive(Debug, Clone, PartialEq)]
pub struct Study {
    /// Study Instance UID.
    pub study_instance_uid: String,
    /// Study date.
    pub study_date: String,
    /// Study time.
    pub study_time: String,
    /// Study description.
    pub study_description: String,
    /// Accession number.
    pub accession_number: String,
    /// Referring physician name.
    pub referring_physician: String,
    /// Contained series.
    pub series: Vec<Series>,
}

/// A DICOM Series: a collection of instances (images).
#[derive(Debug, Clone, PartialEq)]
pub struct Series {
    /// Series Instance UID.
    pub series_instance_uid: String,
    /// Series number.
    pub series_number: Option<i32>,
    /// Series description.
    pub series_description: String,
    /// Modality.
    pub modality: String,
    /// Manufacturer.
    pub manufacturer: String,
    /// Contained instances.
    pub instances: Vec<Instance>,
}

/// A DICOM Instance (a single image or object).
#[derive(Debug, Clone, PartialEq)]
pub struct Instance {
    /// SOP Instance UID.
    pub sop_instance_uid: String,
    /// SOP Class UID.
    pub sop_class_uid: String,
    /// Instance number.
    pub instance_number: Option<i32>,
    /// The raw dataset for this instance.
    pub dataset: Dataset,
}

/// Navigate a DICOM instance dataset and extract its identifying UIDs.
pub fn extract_instance_info(dataset: &Dataset) -> Option<Instance> {
    let sop_instance_uid = match dataset.string(Tag::new(0x0008, 0x0018)) {
        Some(s) => s.into(),
        None => return None,
    };
    let sop_class_uid = match dataset.string(Tag::new(0x0008, 0x0016)) {
        Some(s) => s.into(),
        None => String::new(),
    };
    let instance_number = match dataset.value(Tag::new(0x0020, 0x0013)) {
        Some(Value::I32(v)) => Some(*v),
        Some(Value::String(s)) => s.parse::<i32>().ok(),
        _ => None,
    };

    Some(Instance { sop_instance_uid, sop_class_uid, instance_number, dataset: dataset.clone() })
}

/// Build a Study from a series dataset.
pub fn study_from_series(series_dataset: &Dataset) -> Study {
    Study {
        study_instance_uid: series_dataset.string(Tag::new(0x0020, 0x000D)).unwrap_or("").into(),
        study_date: series_dataset.string(Tag::new(0x0008, 0x0020)).unwrap_or("").into(),
        study_time: series_dataset.string(Tag::new(0x0008, 0x0030)).unwrap_or("").into(),
        study_description: series_dataset.string(Tag::new(0x0010, 0x1030)).unwrap_or("").into(),
        accession_number: series_dataset.string(Tag::new(0x0008, 0x0050)).unwrap_or("").into(),
        referring_physician: series_dataset.string(Tag::new(0x0008, 0x0090)).unwrap_or("").into(),
        series: Vec::new(),
    }
}

/// Build a Series from an instance dataset.
pub fn series_from_instance(instance_dataset: &Dataset) -> Series {
    let series_number = match instance_dataset.value(Tag::new(0x0020, 0x0011)) {
        Some(Value::I32(v)) => Some(*v),
        Some(Value::String(s)) => s.parse::<i32>().ok(),
        _ => None,
    };

    Series {
        series_instance_uid: instance_dataset.string(Tag::new(0x0020, 0x000E)).unwrap_or("").into(),
        series_number,
        series_description: instance_dataset.string(Tag::new(0x0008, 0x103E)).unwrap_or("").into(),
        modality: instance_dataset.string(Tag::new(0x0008, 0x0060)).unwrap_or("").into(),
        manufacturer: instance_dataset.string(Tag::new(0x0008, 0x0070)).unwrap_or("").into(),
        instances: Vec::new(),
    }
}

/// A hierarchy navigator for traversing study/series/instance relationships.
#[derive(Debug, Clone, Default)]
pub struct HierarchyNavigator;

impl HierarchyNavigator {
    /// Create a new navigator.
    pub fn new() -> Self {
        Self
    }

    /// Extract study info from a dataset.
    pub fn study(&self, dataset: &Dataset) -> Option<Study> {
        let uid = dataset.string(Tag::new(0x0020, 0x000D))?;
        let mut study = study_from_series(dataset);
        study.study_instance_uid = uid.into();
        Some(study)
    }

    /// Extract series info from a dataset.
    pub fn series(&self, dataset: &Dataset) -> Option<Series> {
        let uid = dataset.string(Tag::new(0x0020, 0x000E))?;
        let mut series = series_from_instance(dataset);
        series.series_instance_uid = uid.into();
        Some(series)
    }

    /// Extract instance info from a dataset.
    pub fn instance(&self, dataset: &Dataset) -> Option<Instance> {
        extract_instance_info(dataset)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Element, VR};
    use crate::tag::VRCode;

    fn make_test_dataset() -> Dataset {
        let mut ds = Dataset::new();
        ds.push(Element {
            tag: Tag::new(0x0020, 0x000D),
            vr: VR::Known(VRCode::UI),
            value: Value::String("1.2.3.4.5".into()),
        });
        ds.push(Element {
            tag: Tag::new(0x0020, 0x000E),
            vr: VR::Known(VRCode::UI),
            value: Value::String("1.2.3.4.5.6".into()),
        });
        ds.push(Element {
            tag: Tag::new(0x0008, 0x0018),
            vr: VR::Known(VRCode::UI),
            value: Value::String("1.2.3.4.5.6.7".into()),
        });
        ds.push(Element {
            tag: Tag::new(0x0008, 0x0016),
            vr: VR::Known(VRCode::UI),
            value: Value::String("1.2.840.10008.5.1.4.1.1.2".into()),
        });
        ds.push(Element {
            tag: Tag::new(0x0020, 0x0013),
            vr: VR::Known(VRCode::IS),
            value: Value::I32(1),
        });
        ds
    }

    #[test]
    fn test_extract_instance_info() {
        let ds = make_test_dataset();
        let inst = extract_instance_info(&ds).unwrap();
        assert_eq!(inst.sop_instance_uid, "1.2.3.4.5.6.7");
        assert_eq!(inst.sop_class_uid, "1.2.840.10008.5.1.4.1.1.2");
        assert_eq!(inst.instance_number, Some(1));
    }

    #[test]
    fn navigator_study() {
        let ds = make_test_dataset();
        let nav = HierarchyNavigator::new();
        let study = nav.study(&ds).unwrap();
        assert_eq!(study.study_instance_uid, "1.2.3.4.5");
    }

    #[test]
    fn navigator_series() {
        let ds = make_test_dataset();
        let nav = HierarchyNavigator::new();
        let series = nav.series(&ds).unwrap();
        assert_eq!(series.series_instance_uid, "1.2.3.4.5.6");
    }
}
