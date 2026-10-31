//! Integration tests for the ARXML reader.

use tpt_io_arxml::{parse, ArxmlError};

const MODEL: &[u8] = br#"<?xml version="1.0" encoding="UTF-8"?>
<AUTOSAR xmlns="http://autosar.org/schema/r4.0" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance">
  <AR-PACKAGES>
    <AR-PACKAGE UUID="a1">
      <SHORT-NAME>Components</SHORT-NAME>
      <ELEMENTS>
        <APPLICATION-SW-COMPONENT-TYPE UUID="c1">
          <SHORT-NAME>Sensor</SHORT-NAME>
        </APPLICATION-SW-COMPONENT-TYPE>
        <SENSOR-INTERFACE>
          <SHORT-NAME>SpeedIf</SHORT-NAME>
        </SENSOR-INTERFACE>
      </ELEMENTS>
      <AR-PACKAGES>
        <AR-PACKAGE>
          <SHORT-NAME>Fusion</SHORT-NAME>
          <ELEMENTS>
            <APPLICATION-SW-COMPONENT-TYPE>
              <SHORT-NAME>Filter</SHORT-NAME>
            </APPLICATION-SW-COMPONENT-TYPE>
          </ELEMENTS>
        </AR-PACKAGE>
      </AR-PACKAGES>
    </AR-PACKAGE>
    <AR-PACKAGE>
      <SHORT-NAME>Ecu</SHORT-NAME>
    </AR-PACKAGE>
  </AR-PACKAGES>
</AUTOSAR>"#;

#[test]
fn reads_schema_version_and_package_tree() {
    let f = parse(MODEL).unwrap();
    assert_eq!(f.schema_version.as_deref(), Some("r4.0"));
    assert_eq!(f.packages.len(), 2);
    let comp = &f.packages[0];
    assert_eq!(comp.short_name, "Components");
    assert_eq!(comp.uuid.as_deref(), Some("a1"));
    assert_eq!(comp.path, "/Components");
    assert_eq!(comp.packages[0].path, "/Components/Fusion");
    assert_eq!(f.all_packages().len(), 3);
}

#[test]
fn elements_carry_kind_short_name_and_uuid() {
    let f = parse(MODEL).unwrap();
    let elems = &f.packages[0].elements;
    assert_eq!(elems.len(), 2);
    assert_eq!(elems[0].kind, "APPLICATION-SW-COMPONENT-TYPE");
    assert_eq!(elems[0].short_name, "Sensor");
    assert_eq!(elems[0].uuid.as_deref(), Some("c1"));
    assert_eq!(elems[1].kind, "SENSOR-INTERFACE");
    assert!(elems[1].uuid.is_none());
    f.validate().unwrap();
}

#[test]
fn duplicate_short_names_are_rejected() {
    let mut f = parse(MODEL).unwrap();
    f.packages[0].elements[1].short_name = "Sensor".into();
    assert!(matches!(f.validate(), Err(ArxmlError::Invalid(_))));
    let mut f = parse(MODEL).unwrap();
    f.packages[1].short_name = "Components".into();
    assert!(matches!(f.validate(), Err(ArxmlError::Invalid(_))));
}

#[test]
fn malformed_input_is_an_error() {
    assert!(parse(b"<Foo/>").is_err());
    assert!(parse(b"<AUTOSAR><AR-PACKAGES><AR-PACKAGE/></AR-PACKAGES></AUTOSAR>").is_err());
    assert!(parse(b"<AUTOSAR><AR-PACKAGES>").is_err());
}
