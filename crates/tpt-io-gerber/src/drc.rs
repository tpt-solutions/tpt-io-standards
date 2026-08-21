//! Design-rule-check primitive extraction from a parsed [`GerberFile`].
//!
//! PCB design rules are largely encoded in the *apertures* (trace widths, pad
//! sizes, hole diameters) and in X2/X3 design-rule attributes. This module
//! reduces a file to the quantities a DRC engine cares about: the set of trace
//! widths, the annular rings (pad − hole)/2, and any explicit design-rule
//! metadata carried by `%…DR…%` attributes.

use alloc::string::String;
use alloc::vec::Vec;

use crate::aperture::Aperture;
use crate::model::GerberFile;

/// Aggregate design-rule quantities extracted from a Gerber file.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct DrcReport {
    /// Distinct nominal trace/feature widths (mm or inch, per file unit).
    pub trace_widths: Vec<f64>,
    /// Computed annular rings `(outer_width − hole_diameter) / 2` for apertures
    /// that carry a hole (pads/vias).
    pub annular_rings: Vec<f64>,
    /// The smallest annular ring observed, if any.
    pub min_annular_ring: Option<f64>,
    /// The smallest trace width observed, if any.
    pub min_trace_width: Option<f64>,
    /// Explicit design-rule metadata from `%…DR…%` attributes.
    pub design_rules: Vec<String>,
}

impl DrcReport {
    /// Smallest annular ring across all apertures, falling back to `None`.
    pub fn min_annular_ring(&self) -> Option<f64> {
        self.min_annular_ring
    }

    /// Smallest trace width across all apertures, falling back to `None`.
    pub fn min_trace_width(&self) -> Option<f64> {
        self.min_trace_width
    }
}

fn annular_ring(ap: &Aperture) -> Option<f64> {
    match (ap.nominal_width(), ap.hole_diameter()) {
        (Some(w), Some(h)) if h < w => Some((w - h) / 2.0),
        _ => None,
    }
}

/// Extract DRC primitives and design-rule metadata from `file`.
pub fn extract_drc(file: &GerberFile) -> DrcReport {
    let mut report = DrcReport::default();

    for (_, ap) in &file.apertures {
        if let Some(w) = ap.nominal_width() {
            if !report.trace_widths.contains(&w) {
                report.trace_widths.push(w);
            }
        }
        if let Some(ring) = annular_ring(ap) {
            report.annular_rings.push(ring);
        }
    }

    report.min_annular_ring = report.annular_rings.iter().copied().fold(None, |acc, v| match acc {
        None => Some(v),
        Some(m) => Some(m.min(v)),
    });
    report.min_trace_width = report.trace_widths.iter().copied().fold(None, |acc, v| match acc {
        None => Some(v),
        Some(m) => Some(m.min(v)),
    });

    // Design-rule metadata from file and object attributes.
    for dr in file.file_attributes.design_rules() {
        report.design_rules.push(dr.to_string());
    }
    for obj in &file.objects {
        for dr in obj.attributes.design_rules() {
            report.design_rules.push(dr.to_string());
        }
    }

    report
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aperture::Aperture;
    use crate::model::GerberFile;

    #[test]
    fn extracts_widths_and_rings() {
        let mut f = GerberFile::default();
        f.apertures.push((10, Aperture::Circle { diameter: 0.2, hole_diameter: Some(0.1) }));
        f.apertures
            .push((11, Aperture::Rectangle { width: 0.3, height: 0.5, hole_diameter: None }));
        let drc = extract_drc(&f);
        assert!(drc.trace_widths.contains(&0.2));
        assert!(drc.trace_widths.contains(&0.3));
        // (0.2 - 0.1) / 2 = 0.05
        assert_eq!(drc.min_annular_ring, Some(0.05));
        assert_eq!(drc.min_trace_width, Some(0.2));
    }
}
