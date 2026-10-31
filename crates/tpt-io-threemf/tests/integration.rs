//! Integration tests for the 3MF package reader and writer.

use tpt_io_core::zip;
use tpt_io_threemf::{
    model_xml, parse, parse_model_xml, write, Item, Model, Object, ThreeMfError, MODEL_PART,
};

fn tetra(id: u32) -> Object {
    Object {
        id,
        name: None,
        vertices: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
        triangles: vec![[0, 2, 1], [0, 1, 3], [0, 3, 2], [1, 2, 3]],
    }
}

fn model() -> Model {
    Model {
        unit: "millimeter".to_string(),
        objects: vec![Object { name: Some("tetra & friends".to_string()), ..tetra(7) }],
        build: vec![Item {
            object_id: 7,
            transform: Some([1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 5.0, 6.0, 7.0]),
        }],
    }
}

#[test]
fn package_round_trip_keeps_objects_build_items_and_escaped_names() {
    let m = model();
    let bytes = write(&m);
    assert_eq!(parse(&bytes).unwrap(), m);
}

#[test]
fn package_has_the_three_standard_parts() {
    let bytes = write(&model());
    for part in ["[Content_Types].xml", "_rels/.rels", MODEL_PART] {
        assert!(zip::entry(&bytes, part).unwrap().is_some(), "missing part {part}");
    }
}

#[test]
fn model_xml_round_trips_without_the_package() {
    let m = model();
    let xml = model_xml(&m);
    assert_eq!(parse_model_xml(xml.as_bytes()).unwrap(), m);
}

#[test]
fn reads_a_package_with_an_xml_declaration_and_defaults() {
    let xml = br#"<?xml version="1.0" encoding="UTF-8"?>
<model xmlns="http://schemas.microsoft.com/3dmanufacturing/core/2015/02">
  <resources>
    <object id="3" type="model">
      <mesh>
        <vertices>
          <vertex x="0" y="0" z="0"/>
          <vertex x="2" y="0" z="0"/>
          <vertex x="0" y="2" z="0"/>
        </vertices>
        <triangles>
          <triangle v1="0" v2="1" v3="2"/>
        </triangles>
      </mesh>
    </object>
  </resources>
  <build>
    <item objectid="3"/>
  </build>
</model>
"#;
    let package = zip::write_stored(&[(MODEL_PART, xml.as_slice())]);
    let m = parse(&package).unwrap();
    assert_eq!(m.unit, "millimeter");
    assert_eq!(m.objects[0].triangles, vec![[0, 1, 2]]);
    assert_eq!(m.objects[0].vertices[1], [2.0, 0.0, 0.0]);
    assert_eq!(m.build, vec![Item { object_id: 3, transform: None }]);
}

#[test]
fn missing_model_part_is_invalid() {
    let package = zip::write_stored(&[("readme.txt", b"no model here".as_slice())]);
    assert!(matches!(parse(&package), Err(ThreeMfError::Invalid(_))));
}

#[test]
fn triangle_with_missing_vertex_is_invalid() {
    let xml = br#"<model><resources><object id="1"><mesh>
        <vertices><vertex x="0" y="0" z="0"/></vertices>
        <triangles><triangle v1="0" v2="1" v3="2"/></triangles>
        </mesh></object></resources><build/></model>"#;
    assert!(matches!(parse_model_xml(xml), Err(ThreeMfError::Invalid(_))));
}

#[test]
fn build_item_for_unknown_object_is_invalid() {
    let xml = br#"<model><resources/><build><item objectid="9"/></build></model>"#;
    assert!(matches!(parse_model_xml(xml), Err(ThreeMfError::Invalid(_))));
}

#[test]
fn transform_must_have_twelve_values() {
    let xml = br#"<model><resources><object id="1"/></resources><build><item objectid="1" transform="1 0 0 0 1 0 0 0 1"/></build></model>"#;
    assert!(matches!(parse_model_xml(xml), Err(ThreeMfError::Invalid(_))));
}

#[test]
fn non_zip_input_is_a_core_error() {
    assert!(matches!(parse(b"plain text, not a package"), Err(ThreeMfError::Core(_))));
}
