//! Integration tests for the OPC UA NodeSet2 reader.

use tpt_io_opcua::{parse, NodeKind, OpcUaError};

const NODESET: &[u8] = br#"<?xml version="1.0" encoding="utf-8"?>
<UANodeSet xmlns="http://opcfoundation.org/UA/2011/03/UANodeSet.xsd"
           xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance">
  <NamespaceUris>
    <Uri>http://example.org/plant</Uri>
  </NamespaceUris>
  <Aliases>
    <Alias Alias="Organizes">i=35</Alias>
    <Alias Alias="Double">i=11</Alias>
  </Aliases>
  <UAObject NodeId="ns=1;i=1000" BrowseName="1:Plant">
    <DisplayName>Plant</DisplayName>
    <References>
      <Reference ReferenceType="Organizes" IsForward="false">i=85</Reference>
      <Reference ReferenceType="HasTypeDefinition">i=58</Reference>
    </References>
  </UAObject>
  <UAVariable NodeId="ns=1;i=1001" BrowseName="1:Temperature" DataType="Double">
    <DisplayName>Temperature</DisplayName>
    <References>
      <Reference ReferenceType="HasComponent" IsForward="false">ns=1;i=1000</Reference>
    </References>
  </UAVariable>
  <UAMethod NodeId="ns=1;i=1002" BrowseName="1:Reset"/>
  <UAObjectType NodeId="ns=1;i=2000" BrowseName="1:PumpType"/>
</UANodeSet>"#;

#[test]
fn reads_namespaces_and_aliases() {
    let set = parse(NODESET).unwrap();
    assert_eq!(set.namespace_uris, ["http://example.org/plant"]);
    assert_eq!(set.aliases[0], ("Organizes".to_string(), "i=35".to_string()));
    assert_eq!(set.aliases.len(), 2);
}

#[test]
fn reads_typed_nodes_with_references() {
    let set = parse(NODESET).unwrap();
    let kinds: Vec<&NodeKind> = set.nodes.iter().map(|n| &n.kind).collect();
    assert_eq!(
        kinds,
        [&NodeKind::Object, &NodeKind::Variable, &NodeKind::Method, &NodeKind::ObjectType]
    );
    let plant = &set.nodes[0];
    assert_eq!(plant.browse_name, "1:Plant");
    assert_eq!(plant.display_name.as_deref(), Some("Plant"));
    assert_eq!(plant.references[0].reference_type, "Organizes");
    assert!(!plant.references[0].is_forward);
    assert!(plant.references[1].is_forward);
    assert_eq!(plant.references[1].target, "i=58");
    let temp = &set.nodes[1];
    assert_eq!(temp.data_type.as_deref(), Some("Double"));
    set.validate().unwrap();
}

#[test]
fn duplicate_node_ids_are_rejected() {
    let mut set = parse(NODESET).unwrap();
    let copy = set.nodes[0].clone();
    set.nodes.push(copy);
    assert!(matches!(set.validate(), Err(OpcUaError::Invalid(_))));
}

#[test]
fn malformed_nodesets_are_errors() {
    assert!(parse(b"<Foo/>").is_err());
    assert!(parse(b"<UANodeSet><UAObject BrowseName=\"x\"/></UANodeSet>").is_err());
    assert!(parse(b"<UANodeSet><UAObject NodeId=\"i=1\"").is_err());
}

#[test]
fn unknown_elements_are_ignored() {
    let set =
        parse(b"<UANodeSet><Extensions/><UAObject NodeId=\"i=1\" BrowseName=\"x\"/></UANodeSet>")
            .unwrap();
    assert_eq!(set.nodes.len(), 1);
}
