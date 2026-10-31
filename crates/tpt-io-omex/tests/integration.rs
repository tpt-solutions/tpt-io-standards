//! Integration tests for the COMBINE archive reader and writer and the SED-ML summary.

use tpt_io_core::zip;
use tpt_io_omex::sedml::parse_sedml;
use tpt_io_omex::{parse, write, Archive, Content, OmexError};

const SEDML: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<sedML xmlns="http://sed-ml.org/sed-ml/level1/version4" level="1" version="4">
  <listOfModels>
    <model id="m1" language="urn:sedml:language:sbml" source="model.xml"/>
  </listOfModels>
  <listOfSimulations>
    <uniformTimeCourse id="sim1" initialTime="0" outputStartTime="0" outputEndTime="10" numberOfPoints="100">
      <algorithm kisaoID="KISAO:0000019"/>
    </uniformTimeCourse>
  </listOfSimulations>
  <listOfTasks>
    <task id="t1" modelReference="m1" simulationReference="sim1"/>
  </listOfTasks>
  <listOfOutputs>
    <plot2D id="p1"/>
  </listOfOutputs>
</sedML>"#;

fn archive() -> Archive {
    Archive {
        contents: vec![
            Content {
                location: "./model.xml".to_string(),
                format: "http://identifiers.org/combine.specifications/sbml".to_string(),
                master: false,
                data: b"<sbml/>".to_vec(),
            },
            Content {
                location: "./sim.sedml".to_string(),
                format: "http://identifiers.org/combine.specifications/sed-ml".to_string(),
                master: true,
                data: SEDML.as_bytes().to_vec(),
            },
        ],
    }
}

#[test]
fn archive_round_trip_keeps_locations_formats_and_master() {
    let a = archive();
    let back = parse(&write(&a)).unwrap();
    assert_eq!(back, a);
    assert_eq!(back.master().unwrap().location, "./sim.sedml");
    assert_eq!(back.content("./model.xml").unwrap().data, b"<sbml/>");
}

#[test]
fn written_archive_contains_manifest_and_content_files() {
    let bytes = write(&archive());
    assert!(zip::entry(&bytes, "manifest.xml").unwrap().is_some());
    assert!(zip::entry(&bytes, "model.xml").unwrap().is_some());
    assert!(zip::entry(&bytes, "sim.sedml").unwrap().is_some());
}

#[test]
fn manifest_entry_without_its_file_is_invalid() {
    let manifest =
        br#"<omexManifest><content location="./missing.xml" format="x"/></omexManifest>"#;
    let package = zip::write_stored(&[("manifest.xml", manifest.as_slice())]);
    assert!(matches!(parse(&package), Err(OmexError::Invalid(_))));
}

#[test]
fn archive_without_manifest_is_invalid() {
    let package = zip::write_stored(&[("model.xml", b"<sbml/>".as_slice())]);
    assert!(matches!(parse(&package), Err(OmexError::Invalid(_))));
}

#[test]
fn sedml_summary_lists_models_simulations_tasks_and_outputs() {
    let s = parse_sedml(SEDML.as_bytes()).unwrap();
    assert_eq!(s.models[0].id, "m1");
    assert_eq!(s.models[0].source, "model.xml");
    assert_eq!(s.simulations[0].kind, "uniformTimeCourse");
    assert_eq!(s.simulations[0].algorithm.as_deref(), Some("KISAO:0000019"));
    assert_eq!(s.tasks[0].model_ref, "m1");
    assert_eq!(s.tasks[0].simulation_ref, "sim1");
    assert_eq!(s.outputs, vec!["p1".to_string()]);
}

#[test]
fn sedml_with_the_wrong_root_is_invalid() {
    assert!(matches!(parse_sedml(b"<sbml/>"), Err(OmexError::Invalid(_))));
}

#[test]
fn non_zip_input_is_a_core_error() {
    assert!(matches!(parse(b"not an archive"), Err(OmexError::Core(_))));
}
