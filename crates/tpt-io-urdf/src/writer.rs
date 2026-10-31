//! URDF writer. Output re-reads to an equal [`Robot`] when the robot came from URDF.

use alloc::string::String;
use alloc::vec::Vec;
use core::fmt::Write as _;

use tpt_io_core::xml::escape_attr;

use crate::model::{Joint, Link, Pose, Robot};

/// Serialise a robot as URDF.
pub fn write(robot: &Robot) -> Vec<u8> {
    let mut out = String::new();
    out.push_str("<?xml version=\"1.0\"?>\n");
    let _ = writeln!(out, "<robot name=\"{}\">", escape_attr(&robot.name));
    for l in &robot.links {
        link(&mut out, l);
    }
    for j in &robot.joints {
        joint(&mut out, j);
    }
    out.push_str("</robot>\n");
    out.into_bytes()
}

fn link(out: &mut String, l: &Link) {
    let _ = writeln!(out, "  <link name=\"{}\">", escape_attr(&l.name));
    if let Some(m) = l.mass {
        let _ = writeln!(out, "    <inertial><mass value=\"{m}\"/></inertial>");
    }
    for _ in 0..l.visuals {
        out.push_str("    <visual/>\n");
    }
    for _ in 0..l.collisions {
        out.push_str("    <collision/>\n");
    }
    out.push_str("  </link>\n");
}

fn joint(out: &mut String, j: &Joint) {
    let _ = writeln!(
        out,
        "  <joint name=\"{}\" type=\"{}\">",
        escape_attr(&j.name),
        escape_attr(j.kind.as_str())
    );
    let _ = writeln!(out, "    <parent link=\"{}\"/>", escape_attr(&j.parent));
    let _ = writeln!(out, "    <child link=\"{}\"/>", escape_attr(&j.child));
    origin(out, &j.origin);
    let _ = writeln!(out, "    <axis xyz=\"{}\"/>", triple(&j.axis));
    if let Some(l) = &j.limit {
        let _ = writeln!(
            out,
            "    <limit lower=\"{}\" upper=\"{}\" effort=\"{}\" velocity=\"{}\"/>",
            l.lower, l.upper, l.effort, l.velocity
        );
    }
    out.push_str("  </joint>\n");
}

fn origin(out: &mut String, p: &Pose) {
    let _ = writeln!(out, "    <origin xyz=\"{}\" rpy=\"{}\"/>", triple(&p.xyz), triple(&p.rpy));
}

fn triple(v: &[f64; 3]) -> String {
    alloc::format!("{} {} {}", v[0], v[1], v[2])
}
