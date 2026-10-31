//! Reading the 3MF package and its XML model part.

use alloc::string::ToString;
use alloc::vec::Vec;

use tpt_io_core::{xml, zip, ParseError};

use crate::{Item, Model, Object, ThreeMfError, MODEL_PART};

pub(crate) fn parse_package(input: &[u8]) -> Result<Model, ThreeMfError> {
    let part = zip::entry(input, MODEL_PART)?
        .ok_or_else(|| ThreeMfError::Invalid("package has no 3D/3dmodel.model part".to_string()))?;
    parse_model(&part)
}

pub(crate) fn parse_model(input: &[u8]) -> Result<Model, ThreeMfError> {
    let root = xml::parse(input)?;
    if root.local_name() != "model" {
        return Err(ThreeMfError::Invalid("root element is not <model>".to_string()));
    }
    let mut model = Model {
        unit: root.attr("unit").unwrap_or("millimeter").to_string(),
        objects: Vec::new(),
        build: Vec::new(),
    };
    if let Some(resources) = root.child("resources") {
        for el in resources.children_named("object") {
            model.objects.push(object(el)?);
        }
    }
    if let Some(build) = root.child("build") {
        for el in build.children_named("item") {
            let item = item(el)?;
            if model.object(item.object_id).is_none() {
                return Err(ThreeMfError::Invalid(alloc::format!(
                    "build item refers to missing object {}",
                    item.object_id
                )));
            }
            model.build.push(item);
        }
    }
    Ok(model)
}

fn attr<'a>(el: &'a xml::Element, name: &str) -> Result<&'a str, ThreeMfError> {
    el.attr(name).ok_or_else(|| {
        ThreeMfError::Invalid(alloc::format!("<{}> has no {name} attribute", el.name))
    })
}

fn number<T: core::str::FromStr>(el: &xml::Element, name: &str) -> Result<T, ThreeMfError> {
    attr(el, name)?
        .parse::<T>()
        .map_err(|_| ThreeMfError::Core(ParseError::custom(el.offset, "malformed number")))
}

fn object(el: &xml::Element) -> Result<Object, ThreeMfError> {
    let mut obj = Object {
        id: number(el, "id")?,
        name: el.attr("name").map(|s| s.to_string()),
        ..Object::default()
    };
    if let Some(mesh) = el.child("mesh") {
        if let Some(vertices) = mesh.child("vertices") {
            for v in vertices.children_named("vertex") {
                obj.vertices.push([number(v, "x")?, number(v, "y")?, number(v, "z")?]);
            }
        }
        if let Some(triangles) = mesh.child("triangles") {
            for t in triangles.children_named("triangle") {
                let tri: [u32; 3] = [number(t, "v1")?, number(t, "v2")?, number(t, "v3")?];
                if tri.iter().any(|&i| i as usize >= obj.vertices.len()) {
                    return Err(ThreeMfError::Invalid(alloc::format!(
                        "triangle in object {} refers to a missing vertex",
                        obj.id
                    )));
                }
                obj.triangles.push(tri);
            }
        }
    }
    Ok(obj)
}

fn item(el: &xml::Element) -> Result<Item, ThreeMfError> {
    let transform = match el.attr("transform") {
        None => None,
        Some(text) => {
            let values = text
                .split_whitespace()
                .map(|t| t.parse::<f32>())
                .collect::<Result<Vec<f32>, _>>()
                .map_err(|_| {
                    ThreeMfError::Core(ParseError::custom(el.offset, "malformed transform"))
                })?;
            let mut m = [0.0f32; 12];
            if values.len() != 12 {
                return Err(ThreeMfError::Invalid("transform needs exactly 12 values".to_string()));
            }
            m.copy_from_slice(&values);
            Some(m)
        }
    };
    Ok(Item { object_id: number(el, "objectid")?, transform })
}
