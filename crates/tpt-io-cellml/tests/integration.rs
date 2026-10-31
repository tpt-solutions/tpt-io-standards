//! Integration tests for the CellML reader.

use tpt_io_cellml::{parse, CellMlError};

const MODEL: &[u8] = br#"<?xml version="1.0"?>
<model xmlns="http://www.cellml.org/cellml/1.0#" xmlns:cmeta="http://www.cellml.org/metadata/1.0#" name="two_compartment">
  <units name="per_second">
    <unit units="second" exponent="-1"/>
  </units>
  <units name="millimolar"><unit units="mole" prefix="milli"/><unit units="litre" exponent="-1"/></units>
  <component name="plasma">
    <variable name="time" units="second" interface="public_interface"/>
    <variable name="conc" units="millimolar" interface="public_and_private"/>
    <variable name="k" units="per_second"/>
    <math xmlns="http://www.w3.org/1998/Math/MathML"><apply/></math>
  </component>
  <component name="tissue">
    <variable name="conc" units="millimolar" interface="public"/>
  </component>
  <connection>
    <map_components component_1="plasma" component_2="tissue"/>
    <map_variables variable_1="conc" variable_2="conc"/>
  </connection>
</model>"#;

#[test]
fn reads_units_components_variables_and_math_blocks() {
    let m = parse(MODEL).unwrap();
    assert_eq!(m.name, "two_compartment");
    assert_eq!(m.units.len(), 2);
    assert_eq!(m.units[0].terms, [("second".to_string(), -1)]);
    assert_eq!(m.units[1].terms[0], ("mole".to_string(), 1));
    assert_eq!(m.components.len(), 2);
    let plasma = &m.components[0];
    assert_eq!(plasma.variables.len(), 3);
    assert_eq!(plasma.variables[1].interface.as_deref(), Some("public_and_private"));
    assert_eq!(plasma.variables[2].interface, None);
    assert_eq!(plasma.math_blocks, 1);
    assert_eq!(m.components[1].math_blocks, 0);
}

#[test]
fn reads_connections_between_components() {
    let m = parse(MODEL).unwrap();
    let c = &m.connections[0];
    assert_eq!((c.component_1.as_str(), c.component_2.as_str()), ("plasma", "tissue"));
    assert_eq!(c.variables, [("conc".to_string(), "conc".to_string())]);
}

#[test]
fn missing_name_and_wrong_root_are_errors() {
    assert!(matches!(parse(b"<model/>"), Err(CellMlError::Invalid(_))));
    assert!(parse(b"<component name=\"x\"/>").is_err());
    assert!(parse(b"<model name=\"x\"><component>").is_err());
}

#[test]
fn arbitrary_bytes_never_panic() {
    let mut x: u64 = 109;
    for _ in 0..200 {
        let len = (x % 200) as usize;
        let bytes: Vec<u8> = (0..len)
            .map(|_| {
                x ^= x << 13;
                x ^= x >> 7;
                x ^= x << 17;
                let alphabet = b"<model name=\"x\">component variable units unit/> ";
                alphabet[(x % alphabet.len() as u64) as usize]
            })
            .collect();
        let _ = parse(&bytes);
    }
}
