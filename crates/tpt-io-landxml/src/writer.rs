//! LandXML writer: units, project, surfaces with points and faces. Output re-reads to the same model
//! (alignments are written as names only).

use alloc::string::String;
use alloc::vec::Vec;
use core::fmt::Write as _;

use tpt_io_core::xml::escape_attr;

use crate::model::LandXml;

/// Serialise a LandXML document.
pub fn write(land: &LandXml) -> Vec<u8> {
    let mut out = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<LandXML>\n");
    if let Some(unit) = &land.linear_unit {
        let _ = writeln!(out, "  <Units><Metric linearUnit=\"{}\"/></Units>", escape_attr(unit));
    }
    if let Some(p) = &land.project {
        let _ = writeln!(out, "  <Project name=\"{}\"/>", escape_attr(p));
    }
    if !land.surfaces.is_empty() {
        out.push_str("  <Surfaces>\n");
        for s in &land.surfaces {
            let _ = writeln!(out, "    <Surface name=\"{}\">", escape_attr(&s.name));
            out.push_str("      <Definition surfType=\"TIN\">\n        <Pnts>\n");
            for (id, p) in &s.points {
                let _ = writeln!(out, "          <P id=\"{id}\">{} {} {}</P>", p[0], p[1], p[2]);
            }
            out.push_str("        </Pnts>\n        <Faces>\n");
            for f in &s.faces {
                let _ = writeln!(out, "          <F>{} {} {}</F>", f[0], f[1], f[2]);
            }
            out.push_str("        </Faces>\n      </Definition>\n    </Surface>\n");
        }
        out.push_str("  </Surfaces>\n");
    }
    if !land.alignments.is_empty() {
        out.push_str("  <Alignments>\n");
        for a in &land.alignments {
            let _ = writeln!(out, "    <Alignment name=\"{}\"/>", escape_attr(a));
        }
        out.push_str("  </Alignments>\n");
    }
    out.push_str("</LandXML>\n");
    out.into_bytes()
}
