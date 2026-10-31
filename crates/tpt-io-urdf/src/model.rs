//! The common robot model shared by the URDF and SDFormat readers.

use alloc::string::{String, ToString};
use alloc::vec::Vec;

use crate::error::UrdfError;

/// Which XML dialect a robot was read from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dialect {
    /// A `<robot>` document.
    Urdf,
    /// The first `<model>` inside an `<sdf>` document.
    Sdf,
}

/// A rigid body.
#[derive(Debug, Clone, PartialEq)]
pub struct Link {
    /// Link name, unique within the robot.
    pub name: String,
    /// Mass in kilograms, if an inertial block gives one.
    pub mass: Option<f64>,
    /// Number of `<visual>` elements.
    pub visuals: usize,
    /// Number of `<collision>` elements.
    pub collisions: usize,
}

/// A position and orientation: translation in metres and roll/pitch/yaw in radians.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Pose {
    /// Translation `[x, y, z]`.
    pub xyz: [f64; 3],
    /// Rotation `[roll, pitch, yaw]`.
    pub rpy: [f64; 3],
}

/// Joint motion limits.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Limit {
    /// Lower position (or angle) limit.
    pub lower: f64,
    /// Upper position (or angle) limit.
    pub upper: f64,
    /// Maximum effort (force or torque).
    pub effort: f64,
    /// Maximum speed.
    pub velocity: f64,
}

/// The joint types URDF and SDFormat define, plus `Other` for anything else.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JointKind {
    /// No motion between parent and child.
    Fixed,
    /// Rotation with limits.
    Revolute,
    /// Rotation without limits.
    Continuous,
    /// Translation along the axis with limits.
    Prismatic,
    /// Six degrees of freedom (URDF `floating`).
    Floating,
    /// Motion in a plane (URDF `planar`).
    Planar,
    /// Any other type string, kept verbatim (for example SDFormat `ball`).
    Other(String),
}

impl JointKind {
    /// The type string used in the XML.
    pub fn as_str(&self) -> &str {
        match self {
            JointKind::Fixed => "fixed",
            JointKind::Revolute => "revolute",
            JointKind::Continuous => "continuous",
            JointKind::Prismatic => "prismatic",
            JointKind::Floating => "floating",
            JointKind::Planar => "planar",
            JointKind::Other(s) => s,
        }
    }

    /// Parse a type string.
    pub fn from_str_kind(s: &str) -> Self {
        match s {
            "fixed" => JointKind::Fixed,
            "revolute" => JointKind::Revolute,
            "continuous" => JointKind::Continuous,
            "prismatic" => JointKind::Prismatic,
            "floating" => JointKind::Floating,
            "planar" => JointKind::Planar,
            other => JointKind::Other(other.to_string()),
        }
    }
}

/// A joint connecting a parent link to a child link.
#[derive(Debug, Clone, PartialEq)]
pub struct Joint {
    /// Joint name, unique within the robot.
    pub name: String,
    /// Joint type.
    pub kind: JointKind,
    /// Parent link name.
    pub parent: String,
    /// Child link name.
    pub child: String,
    /// Pose of the child frame relative to the parent at zero joint position.
    pub origin: Pose,
    /// Axis of motion in the joint frame. Defaults to `[1, 0, 0]`.
    pub axis: [f64; 3],
    /// Limits, when the joint type has them.
    pub limit: Option<Limit>,
}

/// A robot: its links and joints.
#[derive(Debug, Clone, PartialEq)]
pub struct Robot {
    /// Robot (or model) name.
    pub name: String,
    /// The dialect the robot was read from.
    pub dialect: Dialect,
    /// Links in document order.
    pub links: Vec<Link>,
    /// Joints in document order.
    pub joints: Vec<Joint>,
}

impl Robot {
    /// Check the kinematic tree: unique link and joint names, joints that refer to known links,
    /// and at most one parent per link. Cycles are rejected as well.
    pub fn validate(&self) -> Result<(), UrdfError> {
        let mut names: Vec<&str> = self.links.iter().map(|l| l.name.as_str()).collect();
        names.sort_unstable();
        if names.windows(2).any(|w| w[0] == w[1]) {
            return Err(UrdfError::Invalid("duplicate link name".to_string()));
        }
        let mut joints: Vec<&str> = self.joints.iter().map(|j| j.name.as_str()).collect();
        joints.sort_unstable();
        if joints.windows(2).any(|w| w[0] == w[1]) {
            return Err(UrdfError::Invalid("duplicate joint name".to_string()));
        }
        let mut children: Vec<&str> = Vec::new();
        for j in &self.joints {
            if names.binary_search(&j.parent.as_str()).is_err() {
                return Err(UrdfError::Invalid(alloc::format!("joint '{}' has unknown parent", j.name)));
            }
            if names.binary_search(&j.child.as_str()).is_err() {
                return Err(UrdfError::Invalid(alloc::format!("joint '{}' has unknown child", j.name)));
            }
            if children.contains(&j.child.as_str()) {
                return Err(UrdfError::Invalid(alloc::format!("link '{}' has two parents", j.child)));
            }
            children.push(&j.child);
        }
        // Walk up from every link: a cycle would revisit a link without reaching a root.
        for l in &self.links {
            let mut cur = l.name.as_str();
            let mut steps = 0usize;
            while let Some(j) = self.joints.iter().find(|j| j.child == cur) {
                cur = &j.parent;
                steps += 1;
                if steps > self.links.len() {
                    return Err(UrdfError::Invalid("joint graph contains a cycle".to_string()));
                }
            }
        }
        Ok(())
    }
}
