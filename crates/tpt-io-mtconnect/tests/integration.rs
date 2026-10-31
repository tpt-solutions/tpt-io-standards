//! Integration tests for the MTConnect devices and streams readers.

use tpt_io_mtconnect::{parse_devices, parse_streams, MtConnectError};

const DEVICES: &[u8] = br#"<?xml version="1.0"?>
<MTConnectDevices xmlns="urn:mtconnect.org:MTConnectDevices:1.7">
  <Header creationTime="2024-01-01T00:00:00Z" schemaVersion="1.7"/>
  <Devices>
    <Device id="mill" uuid="M-001" name="Mill">
      <Components>
        <Axes id="axes" name="Axes">
          <DataItems>
            <DataItem id="xpos" category="SAMPLE" type="POSITION" subType="ACTUAL" name="Xact"/>
          </DataItems>
          <Components>
            <Linear id="x" name="X">
              <DataItems>
                <DataItem id="xfeed" category="SAMPLE" type="PATH_FEEDRATE"/>
              </DataItems>
            </Linear>
          </Components>
        </Axes>
        <Controller id="cont" name="Controller">
          <DataItems>
            <DataItem id="exec" category="EVENT" type="EXECUTION"/>
          </DataItems>
        </Controller>
      </Components>
    </Device>
  </Devices>
</MTConnectDevices>"#;

const STREAMS: &[u8] = br#"<MTConnectStreams>
  <Header creationTime="2024-01-01T00:00:01Z"/>
  <Streams>
    <DeviceStream name="Mill" uuid="M-001">
      <ComponentStream component="Axes" componentId="axes" name="Axes">
        <Samples>
          <Position dataItemId="xpos" timestamp="2024-01-01T00:00:01Z" sequence="101" subType="ACTUAL">12.75</Position>
        </Samples>
      </ComponentStream>
      <ComponentStream component="Controller" componentId="cont" name="Controller">
        <Events>
          <Execution dataItemId="exec" timestamp="2024-01-01T00:00:01Z" sequence="102">ACTIVE</Execution>
        </Events>
        <Condition>
          <Normal dataItemId="cond" timestamp="2024-01-01T00:00:02Z" sequence="103"/>
        </Condition>
      </ComponentStream>
    </DeviceStream>
  </Streams>
</MTConnectStreams>"#;

#[test]
fn reads_the_device_tree_with_nested_components() {
    let d = parse_devices(DEVICES).unwrap();
    assert_eq!(d.schema_version.as_deref(), Some("1.7"));
    assert_eq!(d.devices.len(), 1);
    let mill = &d.devices[0];
    assert_eq!((mill.id.as_str(), mill.uuid.as_str(), mill.name.as_str()), ("mill", "M-001", "Mill"));
    assert_eq!(mill.components.len(), 2);
    let axes = &mill.components[0];
    assert_eq!(axes.kind, "Axes");
    assert_eq!(axes.data_items[0].sub_type.as_deref(), Some("ACTUAL"));
    assert_eq!(axes.components[0].kind, "Linear");
    assert_eq!(axes.components[0].data_items[0].kind, "PATH_FEEDRATE");
}

#[test]
fn data_items_are_found_through_nesting() {
    let d = parse_devices(DEVICES).unwrap();
    assert_eq!(d.data_item("xfeed").unwrap().category, "SAMPLE");
    assert!(d.data_item("missing").is_none());
    d.validate().unwrap();
}

#[test]
fn streams_carry_samples_events_and_conditions() {
    let s = parse_streams(STREAMS).unwrap();
    assert_eq!(s.observations.len(), 3);
    let (uuid, first) = &s.observations[0];
    assert_eq!(uuid, "M-001");
    assert_eq!(first.data_item_id, "xpos");
    assert_eq!(first.kind, "Position");
    assert_eq!(first.value, "12.75");
    assert_eq!(first.sequence, Some(101));
    assert_eq!(s.observations[1].1.value, "ACTIVE");
    assert_eq!(s.observations[2].1.kind, "Normal");
    assert_eq!(s.observations[2].1.value, "");
}

#[test]
fn observations_must_match_known_data_items() {
    let d = parse_devices(DEVICES).unwrap();
    let s = parse_streams(STREAMS).unwrap();
    assert_eq!(s.validate_against(&d), Err(MtConnectError::UnknownDataItem("cond".into())));
}

#[test]
fn wrong_root_and_bad_numbers_are_errors() {
    assert!(parse_devices(STREAMS).is_err());
    assert!(parse_streams(DEVICES).is_err());
    let bad = br#"<MTConnectStreams><Streams><DeviceStream uuid="u"><ComponentStream><Samples>
        <Position dataItemId="x" sequence="abc">1</Position></Samples></ComponentStream></DeviceStream></Streams></MTConnectStreams>"#;
    assert!(parse_streams(bad).is_err());
    assert!(parse_devices(b"<MTConnectDevices><Devices><Device/></Devices></MTConnectDevices>").is_err());
}

#[test]
fn duplicate_data_items_are_rejected() {
    let dup = br#"<MTConnectDevices><Devices><Device id="a" uuid="u" name="n"><Components>
        <Controller id="c"><DataItems><DataItem id="x" type="T"/><DataItem id="x" type="T"/></DataItems></Controller>
        </Components></Device></Devices></MTConnectDevices>"#;
    let d = parse_devices(dup).unwrap();
    assert!(d.validate().is_err());
}
