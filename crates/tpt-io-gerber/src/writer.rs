//! Serialization of a [`GerberFile`] back to Gerber (RS-274X) text.
//!
//! The output is canonical: a fixed header (format, unit, polarity), file
//! attributes, macros, apertures, then the graphic objects with aperture/
//! interpolation state re-emitted only when it changes. Re-parsing the output
//! yields a structurally identical [`GerberFile`] (see the round-trip tests).

use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

use crate::aperture::{write_add, write_am};
use crate::coord::{Axis, CoordinateFormat};
use crate::error::GerberError;
use crate::model::{GerberFile, Interpolation, ObjectKind, Polarity, Unit};

/// Serialize a [`GerberFile`] into Gerber (RS-274X) bytes.
pub fn serialize(file: &GerberFile) -> Result<Vec<u8>, GerberError> {
    let mut out = String::new();
    let fmt = file.format;

    // Header: format, unit, polarity.
    out.push_str(&format!(
        "%FS{}{}X{}{}Y{}{}*%\n",
        match fmt.zeros {
            crate::coord::ZeroSuppression::Leading => 'L',
            crate::coord::ZeroSuppression::Trailing => 'T',
        },
        match fmt.notation {
            crate::coord::Notation::Absolute => 'A',
            crate::coord::Notation::Incremental => 'I',
        },
        fmt.x_int,
        fmt.x_dec,
        fmt.y_int,
        fmt.y_dec
    ));
    out.push_str(&format!("%MO{}*%\n", mo_code(file.unit)));
    out.push_str(match file.polarity {
        Polarity::Positive => "%LPD*%\n",
        Polarity::Negative => "%LPC*%\n",
    });

    // File attributes.
    for (name, vals) in &file.file_attributes.entries {
        out.push_str(&format!("%TF.{name},{}*%\n", vals.join(",")));
    }

    // Aperture macros.
    for (_, m) in &file.aperture_macros {
        out.push_str(&write_am(m));
        out.push('\n');
    }

    // Apertures (with aperture attributes immediately preceding).
    for (id, ap) in &file.apertures {
        if let Some((_, attr)) = file.aperture_attributes.iter().find(|(a, _)| a == id) {
            for (name, vals) in &attr.entries {
                out.push_str(&format!("%TA.{name},{}*%\n", vals.join(",")));
            }
        }
        out.push_str(&write_add(*id, ap));
        out.push('\n');
    }

    // Objects.
    let mut last_ap: Option<u32> = None;
    let mut last_interp: Interpolation = Interpolation::Linear;
    let mut last_pos: Option<(f64, f64)> = None;

    for obj in &file.objects {
        for (name, vals) in &obj.attributes.entries {
            out.push_str(&format!("%TO.{name},{}*%\n", vals.join(",")));
        }

        if obj.aperture != last_ap {
            if let Some(ap) = obj.aperture {
                out.push_str(&format!("D{ap}*"));
            }
            last_ap = obj.aperture;
        }

        match &obj.kind {
            ObjectKind::Flash { x, y } => {
                emit_coord(&mut out, fmt, *x, *y);
                out.push_str("D03*");
                last_pos = Some((*x, *y));
            }
            ObjectKind::Line { x0, y0, x1, y1 } => {
                ensure_pos(&mut out, fmt, &mut last_pos, *x0, *y0);
                if last_interp != Interpolation::Linear {
                    out.push_str("G01*");
                    last_interp = Interpolation::Linear;
                }
                emit_coord(&mut out, fmt, *x1, *y1);
                out.push_str("D01*");
                last_pos = Some((*x1, *y1));
            }
            ObjectKind::Arc { x0, y0, x1, y1, i, j, ccw } => {
                ensure_pos(&mut out, fmt, &mut last_pos, *x0, *y0);
                let want =
                    if *ccw { Interpolation::CircularCcw } else { Interpolation::CircularCw };
                if last_interp != want {
                    out.push_str(if *ccw { "G03*" } else { "G02*" });
                    last_interp = want;
                }
                emit_coord(&mut out, fmt, *x1, *y1);
                out.push_str(&format!(
                    "I{}J{}D01*",
                    fmt.encode_axis(Axis::X, *i),
                    fmt.encode_axis(Axis::Y, *j)
                ));
                last_pos = Some((*x1, *y1));
            }
            ObjectKind::Region { points } => {
                if last_interp != Interpolation::Linear {
                    out.push_str("G01*");
                    last_interp = Interpolation::Linear;
                }
                out.push_str("G36*");
                if let Some((sx, sy)) = points.first() {
                    emit_coord(&mut out, fmt, *sx, *sy);
                    out.push_str("D02*");
                    last_pos = Some((*sx, *sy));
                }
                for (px, py) in &points[1..] {
                    emit_coord(&mut out, fmt, *px, *py);
                    out.push_str("D01*");
                    last_pos = Some((*px, *py));
                }
                out.push_str("G37*");
            }
        }
        out.push('\n');
    }

    out.push_str("M02*\n");
    Ok(out.into_bytes())
}

fn mo_code(unit: Unit) -> &'static str {
    match unit {
        Unit::Inch => "IN",
        Unit::Mm => "MM",
    }
}

fn emit_coord(out: &mut String, fmt: CoordinateFormat, x: f64, y: f64) {
    out.push_str(&format!("X{}Y{}", fmt.encode_axis(Axis::X, x), fmt.encode_axis(Axis::Y, y)));
}

/// Emit a `D02` move to `(x, y)` if the current position is not already there.
fn ensure_pos(
    out: &mut String,
    fmt: CoordinateFormat,
    last_pos: &mut Option<(f64, f64)>,
    x: f64,
    y: f64,
) {
    if *last_pos != Some((x, y)) {
        emit_coord(out, fmt, x, y);
        out.push_str("D02*");
        *last_pos = Some((x, y));
    }
}
