//! The MTConnect device and stream model.

use alloc::string::String;
use alloc::vec::Vec;

use crate::error::MtConnectError;

/// A data item: a measurable or reported value of a component.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataItem {
    /// Data item id, unique within the devices document.
    pub id: String,
    /// Name, if given.
    pub name: Option<String>,
    /// `SAMPLE`, `EVENT` or `CONDITION`.
    pub category: String,
    /// The type, e.g. `POSITION` or `EXECUTION`.
    pub kind: String,
    /// The subtype, e.g. `ACTUAL`, if given.
    pub sub_type: Option<String>,
}

/// A component (for example `Axes`, `Controller`, `Path`) with its data items and children.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Component {
    /// Component id.
    pub id: String,
    /// The element name, e.g. `Axes` or `Linear`.
    pub kind: String,
    /// Name, if given.
    pub name: Option<String>,
    /// Data items directly in this component.
    pub data_items: Vec<DataItem>,
    /// Nested components.
    pub components: Vec<Component>,
}

/// A device and its component tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Device {
    /// Device id.
    pub id: String,
    /// Device UUID.
    pub uuid: String,
    /// Device name.
    pub name: String,
    /// Top-level components.
    pub components: Vec<Component>,
}

/// The devices document: every device it describes.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Devices {
    /// The `schemaVersion` from the header, if present.
    pub schema_version: Option<String>,
    /// Devices in document order.
    pub devices: Vec<Device>,
}

impl Devices {
    /// The data item with the given id, searched through every device and component.
    pub fn data_item(&self, id: &str) -> Option<&DataItem> {
        fn find<'a>(cs: &'a [Component], id: &str) -> Option<&'a DataItem> {
            for c in cs {
                if let Some(d) = c.data_items.iter().find(|d| d.id == id) {
                    return Some(d);
                }
                if let Some(d) = find(&c.components, id) {
                    return Some(d);
                }
            }
            None
        }
        self.devices.iter().find_map(|d| find(&d.components, id))
    }

    /// Check that data item ids are unique across the document.
    pub fn validate(&self) -> Result<(), MtConnectError> {
        fn collect<'a>(cs: &'a [Component], out: &mut Vec<&'a str>) {
            for c in cs {
                out.extend(c.data_items.iter().map(|d| d.id.as_str()));
                collect(&c.components, out);
            }
        }
        let mut ids = Vec::new();
        for d in &self.devices {
            collect(&d.components, &mut ids);
        }
        ids.sort_unstable();
        if ids.windows(2).any(|w| w[0] == w[1]) {
            return Err(MtConnectError::Invalid(alloc::string::String::from("duplicate data item id")));
        }
        Ok(())
    }
}

/// One observation from a streams document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Observation {
    /// The data item this observation belongs to.
    pub data_item_id: String,
    /// The element name: `Samples`, `Events` or `Condition` children such as `Position` or `Normal`.
    pub kind: String,
    /// The timestamp as written.
    pub timestamp: String,
    /// The sequence number, if given.
    pub sequence: Option<u64>,
    /// The observation's value text (trimmed).
    pub value: String,
}

/// The streams document: every observation it carries, in document order.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Streams {
    /// The device UUID of each `DeviceStream`, paired with its observations.
    pub observations: Vec<(String, Observation)>,
}

impl Streams {
    /// Check that every observation refers to a data item the devices document defines.
    pub fn validate_against(&self, devices: &Devices) -> Result<(), MtConnectError> {
        for (_, o) in &self.observations {
            if devices.data_item(&o.data_item_id).is_none() {
                return Err(MtConnectError::UnknownDataItem(o.data_item_id.clone()));
            }
        }
        Ok(())
    }
}
