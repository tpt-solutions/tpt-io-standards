//! Integration tests for the AAS JSON reader.

use tpt_io_aas::{parse, AasError};

const ENV: &[u8] = br#"{
  "assetAdministrationShells": [
    {
      "id": "urn:shell:pump-1",
      "idShort": "Pump1",
      "assetInformation": { "assetKind": "Instance", "globalAssetId": "urn:asset:pump-1" },
      "submodels": [
        { "type": "ModelReference", "keys": [ { "type": "Submodel", "value": "urn:sm:nameplate" } ] },
        { "type": "ModelReference", "keys": [ { "type": "Submodel", "value": "urn:sm:external" } ] }
      ]
    }
  ],
  "submodels": [
    {
      "id": "urn:sm:nameplate",
      "idShort": "Nameplate",
      "submodelElements": [
        { "idShort": "Manufacturer", "modelType": "Property" },
        { "idShort": "Parts", "modelType": { "name": "SubmodelElementCollection" } }
      ]
    }
  ],
  "conceptDescriptions": [ { "id": "urn:cd:1" }, { "id": "urn:cd:2" } ]
}"#;

#[test]
fn reads_shells_submodels_and_references() {
    let env = parse(ENV).unwrap();
    assert_eq!(env.shells.len(), 1);
    let shell = &env.shells[0];
    assert_eq!(shell.id_short.as_deref(), Some("Pump1"));
    assert_eq!(shell.global_asset_id.as_deref(), Some("urn:asset:pump-1"));
    assert_eq!(shell.submodel_refs, ["urn:sm:nameplate", "urn:sm:external"]);
    assert_eq!(env.concept_descriptions, 2);
}

#[test]
fn submodel_elements_report_model_type_in_either_form() {
    let env = parse(ENV).unwrap();
    let sm = &env.submodels[0];
    assert_eq!(sm.id_short.as_deref(), Some("Nameplate"));
    assert_eq!(sm.elements[0].model_type, "Property");
    assert_eq!(sm.elements[1].model_type, "SubmodelElementCollection");
    assert_eq!(sm.elements[1].id_short.as_deref(), Some("Parts"));
    env.validate().unwrap();
}

#[test]
fn duplicate_ids_are_rejected() {
    let mut env = parse(ENV).unwrap();
    let copy = env.submodels[0].clone();
    env.submodels.push(copy);
    assert!(matches!(env.validate(), Err(AasError::Invalid(_))));
}

#[test]
fn missing_identifiers_and_bad_json_are_errors() {
    assert!(matches!(parse(br#"{"assetAdministrationShells":[{"idShort":"x"}]}"#), Err(AasError::Invalid(_))));
    assert!(matches!(parse(br#"{"submodels":[{"idShort":"x"}]}"#), Err(AasError::Invalid(_))));
    assert!(parse(b"[1, 2]").is_err());
    assert!(parse(b"{\"assetAdministrationShells\": [").is_err());
}

#[test]
fn an_empty_environment_is_valid() {
    let env = parse(b"{}").unwrap();
    assert!(env.shells.is_empty() && env.submodels.is_empty());
    env.validate().unwrap();
}
