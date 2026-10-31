//! Integration tests for the OpenSCENARIO reader.

use tpt_io_openscenario::{parse, EntityKind, OpenScenarioError};

const SCENARIO: &[u8] = br#"<?xml version="1.0" encoding="utf-8"?>
<OpenSCENARIO>
  <FileHeader revMajor="1" revMinor="1" date="2024-05-01T10:00:00" description="Cut-in" author="test"/>
  <ParameterDeclarations>
    <ParameterDeclaration name="$cutInSpeed" parameterType="double" value="12.5"/>
  </ParameterDeclarations>
  <CatalogLocations>
    <VehicleCatalog><Directory path="../Catalogs/Vehicles"/></VehicleCatalog>
  </CatalogLocations>
  <RoadNetwork>
    <LogicFile filepath="../Roads/highway.xodr"/>
  </RoadNetwork>
  <Entities>
    <ScenarioObject name="Ego">
      <Vehicle name="car" vehicleCategory="car"/>
    </ScenarioObject>
    <ScenarioObject name="Target">
      <Pedestrian name="ped" pedestrianCategory="pedestrian"/>
    </ScenarioObject>
    <ScenarioObject name="Cone">
      <MiscObject name="cone" miscObjectCategory="obstacle"/>
    </ScenarioObject>
  </Entities>
  <Storyboard>
    <Init/>
    <Story name="CutInStory">
      <Act name="CutInAct">
        <ManeuverGroup maximumExecutionCount="1" name="g1"/>
        <ManeuverGroup maximumExecutionCount="1" name="g2"/>
      </Act>
      <Act name="StopAct"/>
    </Story>
  </Storyboard>
</OpenSCENARIO>"#;

#[test]
fn reads_header_parameters_and_catalogs() {
    let s = parse(SCENARIO).unwrap();
    assert_eq!((s.header.rev_major, s.header.rev_minor), (1, 1));
    assert_eq!(s.header.description, "Cut-in");
    assert_eq!(s.parameters[0].name, "$cutInSpeed");
    assert_eq!(s.parameters[0].value, "12.5");
    assert_eq!(s.catalog_locations, ["VehicleCatalog:../Catalogs/Vehicles"]);
    assert_eq!(s.road_network.as_deref(), Some("../Roads/highway.xodr"));
}

#[test]
fn reads_entities_with_kinds_and_categories() {
    let s = parse(SCENARIO).unwrap();
    assert_eq!(s.entities.len(), 3);
    assert_eq!(s.entities[0].kind, EntityKind::Vehicle);
    assert_eq!(s.entities[0].category.as_deref(), Some("car"));
    assert_eq!(s.entities[1].kind, EntityKind::Pedestrian);
    assert_eq!(s.entities[2].kind, EntityKind::MiscObject);
    s.validate().unwrap();
}

#[test]
fn reads_stories_acts_and_maneuver_groups() {
    let s = parse(SCENARIO).unwrap();
    let story = &s.stories[0];
    assert_eq!(story.name, "CutInStory");
    assert_eq!(story.acts.len(), 2);
    assert_eq!(story.acts[0].maneuver_groups, 2);
    assert_eq!(story.acts[1].maneuver_groups, 0);
}

#[test]
fn validation_rejects_other_revisions_and_duplicate_names() {
    let mut s = parse(SCENARIO).unwrap();
    s.header.rev_major = 2;
    assert!(matches!(s.validate(), Err(OpenScenarioError::Invalid(_))));

    let mut s = parse(SCENARIO).unwrap();
    s.entities[1].name = "Ego".into();
    assert!(matches!(s.validate(), Err(OpenScenarioError::Invalid(_))));
}

#[test]
fn malformed_documents_are_errors() {
    assert!(parse(b"<OpenDRIVE/>").is_err());
    assert!(parse(b"<OpenSCENARIO><Storyboard><Story/></Storyboard></OpenSCENARIO>").is_err());
    assert!(parse(b"<OpenSCENARIO><Entities><ScenarioObject/></Entities></OpenSCENARIO>").is_err());
    assert!(parse(b"<OpenSCENARIO><FileHeader revMajor=\"x\"/></OpenSCENARIO>").is_err());
}
