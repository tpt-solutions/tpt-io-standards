//! OpenDRIVE writer. Output re-reads to an equal [`OpenDrive`].

use alloc::string::String;
use alloc::vec::Vec;
use core::fmt::Write as _;

use tpt_io_core::xml::escape_attr;

use crate::model::{Geometry, GeometryKind, Lane, LaneSection, OpenDrive, Road};

/// Serialise a map as OpenDRIVE XML.
pub fn write(map: &OpenDrive) -> Vec<u8> {
    let mut out = String::from("<?xml version=\"1.0\" standalone=\"yes\"?>\n<OpenDRIVE>\n");
    let _ = writeln!(
        out,
        "  <header revMajor=\"{}\" revMinor=\"{}\" name=\"{}\"/>",
        map.header.rev_major,
        map.header.rev_minor,
        escape_attr(&map.header.name)
    );
    for r in &map.roads {
        road(&mut out, r);
    }
    out.push_str("</OpenDRIVE>\n");
    out.into_bytes()
}

fn road(out: &mut String, r: &Road) {
    let junction = r.junction.as_deref().unwrap_or("-1");
    let _ = writeln!(
        out,
        "  <road name=\"{}\" id=\"{}\" length=\"{}\" junction=\"{}\">",
        escape_attr(&r.name),
        escape_attr(&r.id),
        r.length,
        escape_attr(junction)
    );
    out.push_str("    <planView>\n");
    for g in &r.geometry {
        geometry(out, g);
    }
    out.push_str("    </planView>\n    <lanes>\n");
    for ls in &r.lane_sections {
        lane_section(out, ls);
    }
    out.push_str("    </lanes>\n  </road>\n");
}

fn geometry(out: &mut String, g: &Geometry) {
    let _ = writeln!(
        out,
        "      <geometry s=\"{}\" x=\"{}\" y=\"{}\" hdg=\"{}\" length=\"{}\">",
        g.s, g.x, g.y, g.hdg, g.length
    );
    match &g.kind {
        GeometryKind::Line => out.push_str("        <line/>\n"),
        GeometryKind::Arc { curvature } => {
            let _ = writeln!(out, "        <arc curvature=\"{curvature}\"/>");
        }
        GeometryKind::Spiral { curv_start, curv_end } => {
            let _ = writeln!(out, "        <spiral curvStart=\"{curv_start}\" curvEnd=\"{curv_end}\"/>");
        }
        GeometryKind::Poly3 { a, b, c, d } => {
            let _ = writeln!(out, "        <poly3 a=\"{a}\" b=\"{b}\" c=\"{c}\" d=\"{d}\"/>");
        }
        GeometryKind::ParamPoly3 { u, v } => {
            let _ = writeln!(
                out,
                "        <paramPoly3 aU=\"{}\" bU=\"{}\" cU=\"{}\" dU=\"{}\" aV=\"{}\" bV=\"{}\" cV=\"{}\" dV=\"{}\"/>",
                u[0], u[1], u[2], u[3], v[0], v[1], v[2], v[3]
            );
        }
    }
    out.push_str("      </geometry>\n");
}

fn lane_section(out: &mut String, ls: &LaneSection) {
    let _ = writeln!(out, "      <laneSection s=\"{}\">", ls.s);
    side(out, "left", &ls.left);
    side(out, "center", &ls.center);
    side(out, "right", &ls.right);
    out.push_str("      </laneSection>\n");
}

fn side(out: &mut String, name: &str, lanes: &[Lane]) {
    let _ = writeln!(out, "        <{name}>");
    for l in lanes {
        let _ = writeln!(
            out,
            "          <lane id=\"{}\" type=\"{}\" level=\"{}\">",
            l.id,
            escape_attr(&l.kind),
            l.level
        );
        for w in &l.widths {
            let _ = writeln!(
                out,
                "            <width sOffset=\"{}\" a=\"{}\" b=\"{}\" c=\"{}\" d=\"{}\"/>",
                w.s_offset, w.a, w.b, w.c, w.d
            );
        }
        out.push_str("          </lane>\n");
    }
    let _ = writeln!(out, "        </{name}>");
}
