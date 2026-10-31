//! Readers for the MTConnect devices and streams documents.

use alloc::string::{String, ToString};
use alloc::vec::Vec;

use tpt_io_core::xml::{self, Element};
use tpt_io_core::ParseError;

use crate::error::MtConnectError;
use crate::model::{Component, DataItem, Device, Devices, Observation, Streams};

/// Parse an `MTConnectDevices` document.
pub fn parse_devices(input: &[u8]) -> Result<Devices, MtConnectError> {
    let root = xml::parse(input)?;
    if root.local_name() != "MTConnectDevices" {
        return Err(
            ParseError::custom(root.offset, "root element is not <MTConnectDevices>").into()
        );
    }
    let schema_version =
        root.child("Header").and_then(|h| h.attr("schemaVersion")).map(|s| s.to_string());
    let mut devices = Vec::new();
    if let Some(list) = root.child("Devices") {
        for d in list.children_named("Device") {
            devices.push(device(d)?);
        }
    }
    Ok(Devices { schema_version, devices })
}

fn device(el: &Element) -> Result<Device, MtConnectError> {
    let components = match el.child("Components") {
        Some(list) => list.children.iter().map(component).collect::<Result<_, _>>()?,
        None => Vec::new(),
    };
    Ok(Device {
        id: required(el, "id")?,
        uuid: el.attr("uuid").unwrap_or("").to_string(),
        name: el.attr("name").unwrap_or("").to_string(),
        components,
    })
}

fn component(el: &Element) -> Result<Component, MtConnectError> {
    let mut data_items = Vec::new();
    if let Some(list) = el.child("DataItems") {
        for d in list.children_named("DataItem") {
            data_items.push(data_item(d)?);
        }
    }
    let mut components = Vec::new();
    if let Some(list) = el.child("Components") {
        for c in &list.children {
            components.push(component(c)?);
        }
    }
    Ok(Component {
        id: required(el, "id")?,
        kind: el.local_name().to_string(),
        name: el.attr("name").map(|s| s.to_string()),
        data_items,
        components,
    })
}

fn data_item(el: &Element) -> Result<DataItem, MtConnectError> {
    Ok(DataItem {
        id: required(el, "id")?,
        name: el.attr("name").map(|s| s.to_string()),
        category: el.attr("category").unwrap_or("").to_string(),
        kind: required(el, "type")?,
        sub_type: el.attr("subType").map(|s| s.to_string()),
    })
}

/// Parse an `MTConnectStreams` document.
pub fn parse_streams(input: &[u8]) -> Result<Streams, MtConnectError> {
    let root = xml::parse(input)?;
    if root.local_name() != "MTConnectStreams" {
        return Err(
            ParseError::custom(root.offset, "root element is not <MTConnectStreams>").into()
        );
    }
    let mut observations = Vec::new();
    if let Some(streams) = root.child("Streams") {
        for ds in streams.children_named("DeviceStream") {
            let uuid = ds.attr("uuid").unwrap_or("").to_string();
            for cs in ds.children_named("ComponentStream") {
                for group in &cs.children {
                    if !matches!(group.local_name(), "Samples" | "Events" | "Condition") {
                        continue;
                    }
                    for obs in &group.children {
                        observations.push((uuid.clone(), observation(obs)?));
                    }
                }
            }
        }
    }
    Ok(Streams { observations })
}

fn observation(el: &Element) -> Result<Observation, MtConnectError> {
    let sequence = match el.attr("sequence") {
        Some(s) => Some(
            s.parse::<u64>()
                .map_err(|_| ParseError::custom(el.offset, "sequence is not an integer"))?,
        ),
        None => None,
    };
    Ok(Observation {
        data_item_id: required(el, "dataItemId")?,
        kind: el.local_name().to_string(),
        timestamp: el.attr("timestamp").unwrap_or("").to_string(),
        sequence,
        value: el.text_trimmed().to_string(),
    })
}

fn required(el: &Element, name: &str) -> Result<String, MtConnectError> {
    el.attr(name)
        .map(|s| s.to_string())
        .ok_or_else(|| ParseError::custom(el.offset, "required attribute is missing").into())
}
