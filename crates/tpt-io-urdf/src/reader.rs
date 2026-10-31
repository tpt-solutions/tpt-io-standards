//! Reader for URDF and SDFormat documents.

use alloc::string::{String, ToString};
use alloc::vec::Vec;

use tpt_io_core::xml::{self, Element};
use tpt_io_core::ParseError;

use crate::error::UrdfError;
use crate::model::{Dialect, Joint, JointKind, Limit, Link, Pose, Robot};

/// Parse a URDF (`<robot>`) or SDFormat (`<sdf>`) document.
pub fn parse(input: &[u8]) -> Result<Robot, UrdfError> {
    let root = xml::parse(input)?;
    match root.local_name() {
        "robot" => Ok(robot(&root, Dialect::Urdf)?),
        "sdf" => {
            let model = root
                .child("model")
                .ok_or_else(|| UrdfError::Invalid("sdf document has no <model>".to_string()))?;
            Ok(robot(model, Dialect::Sdf)?)
        }
        _ => Err(ParseError::custom(root.offset, "root element is not <robot> or <sdf>").into()),
    }
}

fn robot(el: &Element, dialect: Dialect) -> Result<Robot, UrdfError> {
    let name = el
        .attr("name")
        .ok_or_else(|| ParseError::custom(el.offset, "robot or model has no name"))?
        .to_string();
    let mut links = Vec::new();
    for l in el.children_named("link") {
        links.push(link(l, dialect)?);
    }
    let mut joints = Vec::new();
    for j in el.children_named("joint") {
        joints.push(joint(j, dialect)?);
    }
    Ok(Robot { name, dialect, links, joints })
}

fn link(el: &Element, dialect: Dialect) -> Result<Link, UrdfError> {
    let name = required_attr(el, "name")?;
    let mass = match dialect {
        Dialect::Urdf => el
            .path(&["inertial", "mass"])
            .and_then(|m| m.attr("value"))
            .map(|v| number(v, el.offset))
            .transpose()?,
        Dialect::Sdf => el
            .path(&["inertial", "mass"])
            .map(|m| number(m.text_trimmed(), el.offset))
            .transpose()?,
    };
    Ok(Link {
        name,
        mass,
        visuals: el.children_named("visual").count(),
        collisions: el.children_named("collision").count(),
    })
}

fn joint(el: &Element, dialect: Dialect) -> Result<Joint, UrdfError> {
    let name = required_attr(el, "name")?;
    let kind = JointKind::from_str_kind(&required_attr(el, "type")?);
    let (parent, child) = match dialect {
        Dialect::Urdf => (
            el.child("parent").and_then(|p| p.attr("link")),
            el.child("child").and_then(|c| c.attr("link")),
        ),
        Dialect::Sdf => (
            el.child("parent").map(|p| p.text_trimmed()),
            el.child("child").map(|c| c.text_trimmed()),
        ),
    };
    let parent = parent
        .ok_or_else(|| UrdfError::Invalid(alloc::format!("joint '{name}' has no parent")))?
        .to_string();
    let child = child
        .ok_or_else(|| UrdfError::Invalid(alloc::format!("joint '{name}' has no child")))?
        .to_string();

    let origin = match dialect {
        Dialect::Urdf => match el.child("origin") {
            Some(o) => Pose {
                xyz: vector(o.attr("xyz").unwrap_or("0 0 0"), o.offset)?,
                rpy: vector(o.attr("rpy").unwrap_or("0 0 0"), o.offset)?,
            },
            None => Pose::default(),
        },
        Dialect::Sdf => match el.child("pose") {
            Some(p) => sdf_pose(p.text_trimmed(), p.offset)?,
            None => Pose::default(),
        },
    };

    let axis_el = el.child("axis");
    let axis = match (dialect, axis_el) {
        (Dialect::Urdf, Some(a)) => vector(a.attr("xyz").unwrap_or("1 0 0"), a.offset)?,
        (Dialect::Sdf, Some(a)) => match a.child("xyz") {
            Some(x) => vector(x.text_trimmed(), x.offset)?,
            None => [1.0, 0.0, 0.0],
        },
        _ => [1.0, 0.0, 0.0],
    };

    let limit = match dialect {
        Dialect::Urdf => match el.child("limit") {
            Some(l) => Some(limit_from(
                |n| l.attr(n),
                l.offset,
            )?),
            None => None,
        },
        Dialect::Sdf => match axis_el.and_then(|a| a.child("limit")) {
            Some(l) => Some(limit_from(
                |n| l.child(n).map(|c| c.text_trimmed()),
                l.offset,
            )?),
            None => None,
        },
    };

    Ok(Joint { name, kind, parent, child, origin, axis, limit })
}

/// Build a limit from named values; a missing value defaults to zero.
fn limit_from<'a>(get: impl Fn(&str) -> Option<&'a str>, at: usize) -> Result<Limit, UrdfError> {
    let value = |name: &str| get(name).map(|v| number(v, at)).transpose().map(|v| v.unwrap_or(0.0));
    Ok(Limit {
        lower: value("lower")?,
        upper: value("upper")?,
        effort: value("effort")?,
        velocity: value("velocity")?,
    })
}

fn required_attr(el: &Element, name: &str) -> Result<String, UrdfError> {
    el.attr(name)
        .map(|s| s.to_string())
        .ok_or_else(|| ParseError::custom(el.offset, "required attribute is missing").into())
}

fn number(s: &str, at: usize) -> Result<f64, UrdfError> {
    s.trim()
        .parse::<f64>()
        .map_err(|_| ParseError::custom(at, "expected a number").into())
}

/// Three whitespace-separated numbers.
fn vector(s: &str, at: usize) -> Result<[f64; 3], UrdfError> {
    let v: Vec<f64> = s
        .split_whitespace()
        .map(|t| number(t, at))
        .collect::<Result<_, _>>()?;
    match v.as_slice() {
        [a, b, c] => Ok([*a, *b, *c]),
        _ => Err(ParseError::custom(at, "expected three numbers").into()),
    }
}

/// An SDFormat pose: six numbers `x y z roll pitch yaw`.
fn sdf_pose(s: &str, at: usize) -> Result<Pose, UrdfError> {
    let v: Vec<f64> = s
        .split_whitespace()
        .map(|t| number(t, at))
        .collect::<Result<_, _>>()?;
    match v.as_slice() {
        [] => Ok(Pose::default()),
        [x, y, z, r, p, yaw] => Ok(Pose { xyz: [*x, *y, *z], rpy: [*r, *p, *yaw] }),
        [x, y, z] => Ok(Pose { xyz: [*x, *y, *z], rpy: [0.0; 3] }),
        _ => Err(ParseError::custom(at, "SDFormat pose needs 3 or 6 numbers").into()),
    }
}
