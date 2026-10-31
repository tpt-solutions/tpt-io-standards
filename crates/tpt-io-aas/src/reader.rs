//! Reader for AAS JSON environments.

use alloc::string::{String, ToString};
use alloc::vec::Vec;

use tpt_io_core::{parse_json, Json, ParseError};

use crate::error::AasError;
use crate::model::{Environment, Shell, Submodel, SubmodelElement};

/// Parse an AAS environment in JSON form.
pub fn parse(input: &[u8]) -> Result<Environment, AasError> {
    let root = parse_json(input)?;
    if !matches!(root, Json::Object(_)) {
        return Err(ParseError::custom(0, "environment must be a JSON object").into());
    }
    let mut env = Environment::default();
    for s in items(&root, "assetAdministrationShells") {
        env.shells.push(shell(s)?);
    }
    for s in items(&root, "submodels") {
        env.submodels.push(submodel(s)?);
    }
    env.concept_descriptions = items(&root, "conceptDescriptions").count();
    Ok(env)
}

fn items<'a>(root: &'a Json, key: &str) -> impl Iterator<Item = &'a Json> + 'a {
    root.get(key).and_then(|v| v.as_array()).unwrap_or(&[]).iter()
}

fn text(v: &Json, key: &str) -> Option<String> {
    v.get(key).and_then(|x| x.as_str()).map(|s| s.to_string())
}

fn shell(v: &Json) -> Result<Shell, AasError> {
    let id = text(v, "id").ok_or_else(|| AasError::Invalid("shell has no id".to_string()))?;
    let global_asset_id = v
        .get("assetInformation")
        .and_then(|a| text(a, "globalAssetId"));
    let submodel_refs = match v.get("submodels").and_then(|s| s.as_array()) {
        Some(refs) => refs
            .iter()
            .filter_map(|r| r.get("keys").and_then(|k| k.as_array()).and_then(|k| k.last()))
            .filter_map(|k| k.get("value").and_then(|x| x.as_str()).map(|s| s.to_string()))
            .collect(),
        None => Vec::new(),
    };
    Ok(Shell { id, id_short: text(v, "idShort"), global_asset_id, submodel_refs })
}

fn submodel(v: &Json) -> Result<Submodel, AasError> {
    let id = text(v, "id").ok_or_else(|| AasError::Invalid("submodel has no id".to_string()))?;
    let elements = items(v, "submodelElements")
        .map(|e| SubmodelElement {
            id_short: text(e, "idShort"),
            model_type: model_type(e),
        })
        .collect();
    Ok(Submodel { id, id_short: text(v, "idShort"), elements })
}

/// `modelType` is a string in most files and an object with a `name` in later revisions.
fn model_type(e: &Json) -> String {
    match e.get("modelType") {
        Some(Json::Str(s)) => s.clone(),
        Some(obj) => text(obj, "name").unwrap_or_default(),
        None => String::new(),
    }
}
