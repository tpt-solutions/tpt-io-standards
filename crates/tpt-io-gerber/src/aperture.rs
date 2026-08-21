//! Aperture definitions (`%ADD…%`) and aperture macros (`%AM…%`) for Gerber.
//!
//! Standard apertures are circles, rectangles, obrounds, and regular polygons,
//! each optionally carrying a central hole. Aperture macros describe
//! user-defined shapes built from primitives (circle, vector line, outline,
//! polygon, thermal, …) referenced by name.

use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

use crate::error::GerberError;

/// A standard or macro-referencing aperture.
#[derive(Debug, Clone, PartialEq)]
pub enum Aperture {
    /// Circular aperture of `diameter`, optionally with a central `hole_diameter`.
    Circle {
        /// Outer diameter.
        diameter: f64,
        /// Central hole diameter, if any.
        hole_diameter: Option<f64>,
    },
    /// Rectangular aperture.
    Rectangle {
        /// Width (X extent).
        width: f64,
        /// Height (Y extent).
        height: f64,
        /// Central hole diameter, if any.
        hole_diameter: Option<f64>,
    },
    /// Obround (stadium) aperture.
    Obround {
        /// Width (X extent).
        width: f64,
        /// Height (Y extent).
        height: f64,
        /// Central hole diameter, if any.
        hole_diameter: Option<f64>,
    },
    /// Regular polygon aperture.
    Polygon {
        /// Outer diameter (vertex circle).
        diameter: f64,
        /// Number of vertices (≥ 3).
        vertices: u32,
        /// Rotation in degrees.
        rotation: f64,
        /// Central hole diameter, if any.
        hole_diameter: Option<f64>,
    },
    /// User-defined macro aperture referencing a named [`ApertureMacro`].
    Macro {
        /// Referenced macro name.
        name: String,
        /// Modifier parameters passed to the macro.
        params: Vec<f64>,
    },
}

impl Aperture {
    /// The characteristic width of the aperture: for a circle this is its
    /// diameter; for rectangles/obrounds the smaller of width and height; for
    /// polygons their diameter; for macros `None` (shape-dependent).
    pub fn nominal_width(&self) -> Option<f64> {
        match self {
            Aperture::Circle { diameter, .. } => Some(*diameter),
            Aperture::Rectangle { width, height, .. } => Some(width.min(*height)),
            Aperture::Obround { width, height, .. } => Some(width.min(*height)),
            Aperture::Polygon { diameter, .. } => Some(*diameter),
            Aperture::Macro { .. } => None,
        }
    }

    /// The central hole diameter, if the aperture has one.
    pub fn hole_diameter(&self) -> Option<f64> {
        match self {
            Aperture::Circle { hole_diameter, .. }
            | Aperture::Rectangle { hole_diameter, .. }
            | Aperture::Obround { hole_diameter, .. }
            | Aperture::Polygon { hole_diameter, .. } => *hole_diameter,
            Aperture::Macro { .. } => None,
        }
    }

    /// Emit the `%ADD…%` modifier text (everything after the type letter).
    fn modifiers(&self) -> String {
        let mut parts: Vec<String> = Vec::new();
        match self {
            Aperture::Circle { diameter, hole_diameter } => {
                parts.push(diameter.to_string());
                if let Some(h) = hole_diameter {
                    parts.push(h.to_string());
                }
            }
            Aperture::Rectangle { width, height, hole_diameter }
            | Aperture::Obround { width, height, hole_diameter } => {
                parts.push(width.to_string());
                parts.push(height.to_string());
                if let Some(h) = hole_diameter {
                    parts.push(h.to_string());
                }
            }
            Aperture::Polygon { diameter, vertices, rotation, hole_diameter } => {
                parts.push(diameter.to_string());
                parts.push(vertices.to_string());
                parts.push(rotation.to_string());
                if let Some(h) = hole_diameter {
                    parts.push(h.to_string());
                }
            }
            Aperture::Macro { name: _, params } => {
                for p in params {
                    parts.push(p.to_string());
                }
            }
        }
        parts.join(&sep().to_string())
    }

    /// Single-character type code used in `%ADD…%` definitions.
    fn type_code(&self) -> char {
        match self {
            Aperture::Circle { .. } => 'C',
            Aperture::Rectangle { .. } => 'R',
            Aperture::Obround { .. } => 'O',
            Aperture::Polygon { .. } => 'P',
            Aperture::Macro { .. } => 'M',
        }
    }
}

fn sep() -> char {
    'X'
}

/// Parse an aperture definition block (without the surrounding `%…%`),
/// e.g. `ADD10C,0.5` or `ADD11R,1.0X2.0`.
pub fn parse_add(s: &str) -> Result<(u32, Aperture), GerberError> {
    let b = s.as_bytes();
    if b.len() < 3 || !b[..2].eq_ignore_ascii_case(b"AD") {
        return Err(GerberError::invalid_aperture(0, s.to_string()));
    }
    let mut i = 2usize;
    if b.get(i).map(|c| c.to_ascii_uppercase()) != Some(b'D') {
        return Err(GerberError::invalid_aperture(i, s.to_string()));
    }
    i += 1;
    let start = i;
    while i < b.len() && b[i].is_ascii_digit() {
        i += 1;
    }
    let id: u32 =
        s[start..i].parse().map_err(|_| GerberError::invalid_aperture(start, s.to_string()))?;
    if id == 0 {
        return Err(GerberError::invalid_aperture(start, s.to_string()));
    }

    // Type letter, then modifiers after a comma.
    let ty = match b.get(i) {
        Some(&c) => c.to_ascii_uppercase(),
        None => return Err(GerberError::invalid_aperture(i, s.to_string())),
    };
    i += 1;
    let rest = if b.get(i) == Some(&b',') { &s[i + 1..] } else { "" };
    let mods: Vec<&str> = if rest.is_empty() { Vec::new() } else { rest.split('X').collect() };

    let aperture = match ty {
        b'C' => Aperture::Circle {
            diameter: num(&mods, 0, s)?,
            hole_diameter: mods.get(1).and_then(|v| parse_opt(v)),
        },
        b'R' => Aperture::Rectangle {
            width: num(&mods, 0, s)?,
            height: num(&mods, 1, s)?,
            hole_diameter: mods.get(2).and_then(|v| parse_opt(v)),
        },
        b'O' => Aperture::Obround {
            width: num(&mods, 0, s)?,
            height: num(&mods, 1, s)?,
            hole_diameter: mods.get(2).and_then(|v| parse_opt(v)),
        },
        b'P' => Aperture::Polygon {
            diameter: num(&mods, 0, s)?,
            vertices: num(&mods, 1, s)?.round().max(3.0) as u32,
            rotation: mods.get(2).and_then(|v| parse_opt(v)).unwrap_or(0.0),
            hole_diameter: mods.get(3).and_then(|v| parse_opt(v)),
        },
        b'M' => {
            // Macro aperture: the type letters after 'M' form the macro name,
            // then optional `,params`.
            let after = &s[i..];
            let (name, params) = match after.split_once(',') {
                Some((n, p)) => (n.to_string(), split_floats(p)),
                None => (after.to_string(), Vec::new()),
            };
            Aperture::Macro { name, params }
        }
        _ => return Err(GerberError::invalid_aperture(i, s.to_string())),
    };

    Ok((id, aperture))
}

fn parse_opt(v: &str) -> Option<f64> {
    v.trim().parse::<f64>().ok()
}

fn num(mods: &[&str], idx: usize, src: &str) -> Result<f64, GerberError> {
    mods.get(idx)
        .and_then(|v| v.trim().parse::<f64>().ok())
        .ok_or_else(|| GerberError::invalid_aperture(0, src.to_string()))
}

fn split_floats(s: &str) -> Vec<f64> {
    s.split('X').filter_map(|v| v.trim().parse::<f64>().ok()).collect()
}

/// Serialize an aperture definition as `%ADD{id}{type},{modifiers}*%`.
pub fn write_add(id: u32, aperture: &Aperture) -> String {
    let mods = aperture.modifiers();
    match &aperture {
        Aperture::Macro { name, .. } => {
            let p = if mods.is_empty() { String::new() } else { format!(",{mods}") };
            format!("%ADD{id}M{name}{p}*%")
        }
        _ => format!("%ADD{id}{}{}*%", aperture.type_code(), comma_mods(&mods)),
    }
}

fn comma_mods(mods: &str) -> String {
    if mods.is_empty() {
        String::new()
    } else {
        format!(",{mods}")
    }
}

/// A single primitive inside an aperture macro.
#[derive(Debug, Clone, PartialEq)]
pub enum MacroPrimitive {
    /// Circle: (exposure, diameter, center_x, center_y).
    Circle {
        /// `1` = draw, `0` = no draw.
        exposure: u8,
        /// Diameter.
        diameter: f64,
        /// Center X.
        center_x: f64,
        /// Center Y.
        center_y: f64,
    },
    /// Vector line: (exposure, width, start_x, start_y, end_x, end_y, rotation).
    VectorLine {
        /// `1` = draw, `0` = no draw.
        exposure: u8,
        /// Line width.
        width: f64,
        /// Start X.
        start_x: f64,
        /// Start Y.
        start_y: f64,
        /// End X.
        end_x: f64,
        /// End Y.
        end_y: f64,
        /// Rotation in degrees.
        rotation: f64,
    },
    /// Closed outline: (exposure, [(x,y), …]).
    Outline {
        /// `1` = draw, `0` = no draw.
        exposure: u8,
        /// Vertices in order.
        points: Vec<(f64, f64)>,
    },
    /// Regular polygon: (exposure, diameter, vertices, rotation, center_x, center_y).
    Polygon {
        /// `1` = draw, `0` = no draw.
        exposure: u8,
        /// Outer diameter.
        diameter: f64,
        /// Number of vertices.
        vertices: u32,
        /// Rotation in degrees.
        rotation: f64,
        /// Center X.
        center_x: f64,
        /// Center Y.
        center_y: f64,
    },
    /// Thermal (moat) primitive.
    Thermal {
        /// Center X.
        center_x: f64,
        /// Center Y.
        center_y: f64,
        /// Outer diameter.
        outer_diameter: f64,
        /// Inner diameter.
        inner_diameter: f64,
        /// Gap width.
        gap: f64,
    },
    /// A primitive the parser chose not to special-case (code + raw fields).
    Other {
        /// Primitive code.
        code: u32,
        /// Numeric fields verbatim.
        fields: Vec<f64>,
    },
}

impl MacroPrimitive {
    /// Serialize this primitive back into its `code,param,param,…` form.
    pub fn to_text(&self) -> String {
        match self {
            MacroPrimitive::Circle { exposure, diameter, center_x, center_y } => {
                format!("1,{exposure},{diameter},{center_x},{center_y}")
            }
            MacroPrimitive::VectorLine {
                exposure,
                width,
                start_x,
                start_y,
                end_x,
                end_y,
                rotation,
            } => {
                format!("2,{exposure},{width},{start_x},{start_y},{end_x},{end_y},{rotation}")
            }
            MacroPrimitive::Outline { exposure, points } => {
                let mut s = format!("4,{}", points.len() + 1);
                for (x, y) in points {
                    s.push_str(&format!(",{x},{y}"));
                }
                // Closing point repeats the first.
                if let Some((x, y)) = points.first() {
                    s.push_str(&format!(",{x},{y}"));
                }
                s.push_str(&format!(",{exposure}"));
                s
            }
            MacroPrimitive::Polygon {
                exposure,
                diameter,
                vertices,
                rotation,
                center_x,
                center_y,
            } => {
                format!("5,{exposure},{diameter},{vertices},{rotation},{center_x},{center_y}")
            }
            MacroPrimitive::Thermal { center_x, center_y, outer_diameter, inner_diameter, gap } => {
                format!("6,{center_x},{center_y},{outer_diameter},{inner_diameter},{gap}")
            }
            MacroPrimitive::Other { code, fields } => {
                let mut s = code.to_string();
                for f in fields {
                    s.push_str(&format!(",{f}"));
                }
                s
            }
        }
    }
}

/// A named aperture macro.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ApertureMacro {
    /// Macro name (referenced by macro apertures).
    pub name: String,
    /// Primitives in declaration order.
    pub primitives: Vec<MacroPrimitive>,
}

/// Parse an aperture-macro block (without the surrounding `%…%`),
/// e.g. `AMRECT*4,0,0,0.5,1.0,0.0*4,0,0,0.5,1.0,90.0`.
pub fn parse_am(s: &str) -> Result<ApertureMacro, GerberError> {
    let rest = s
        .strip_prefix("AM")
        .or_else(|| s.strip_prefix("am"))
        .ok_or_else(|| GerberError::invalid_aperture(0, s.to_string()))?;
    let mut parts = rest.split('*');
    let name = parts.next().unwrap_or("").to_string();
    let mut primitives = Vec::new();
    for raw in parts {
        if raw.is_empty() {
            continue; // final terminator
        }
        primitives.push(
            parse_primitive(raw).map_err(|_| GerberError::invalid_aperture(0, raw.to_string()))?,
        );
    }
    if primitives.is_empty() {
        return Err(GerberError::invalid_aperture(0, s.to_string()));
    }
    Ok(ApertureMacro { name, primitives })
}

fn parse_primitive(s: &str) -> Result<MacroPrimitive, GerberError> {
    let f: Vec<f64> = s.split(',').filter_map(|v| v.trim().parse::<f64>().ok()).collect();
    let code =
        f.first().copied().ok_or_else(|| GerberError::invalid_aperture(0, s.to_string()))? as u32;
    Ok(match code {
        1 => MacroPrimitive::Circle {
            exposure: f.get(1).copied().unwrap_or(0.0) as u8,
            diameter: f.get(2).copied().unwrap_or(0.0),
            center_x: f.get(3).copied().unwrap_or(0.0),
            center_y: f.get(4).copied().unwrap_or(0.0),
        },
        2 => MacroPrimitive::VectorLine {
            exposure: f.get(1).copied().unwrap_or(0.0) as u8,
            width: f.get(2).copied().unwrap_or(0.0),
            start_x: f.get(3).copied().unwrap_or(0.0),
            start_y: f.get(4).copied().unwrap_or(0.0),
            end_x: f.get(5).copied().unwrap_or(0.0),
            end_y: f.get(6).copied().unwrap_or(0.0),
            rotation: f.get(7).copied().unwrap_or(0.0),
        },
        4 => {
            let count = f.get(1).copied().unwrap_or(0.0) as usize;
            let mut points = Vec::new();
            let mut idx = 2;
            for _ in 0..count.saturating_sub(1) {
                if let (Some(x), Some(y)) = (f.get(idx), f.get(idx + 1)) {
                    points.push((*x, *y));
                }
                idx += 2;
            }
            MacroPrimitive::Outline { exposure: f.last().copied().unwrap_or(0.0) as u8, points }
        }
        5 => MacroPrimitive::Polygon {
            exposure: f.get(1).copied().unwrap_or(0.0) as u8,
            diameter: f.get(2).copied().unwrap_or(0.0),
            vertices: f.get(3).copied().unwrap_or(3.0) as u32,
            rotation: f.get(4).copied().unwrap_or(0.0),
            center_x: f.get(5).copied().unwrap_or(0.0),
            center_y: f.get(6).copied().unwrap_or(0.0),
        },
        6 => MacroPrimitive::Thermal {
            center_x: f.get(1).copied().unwrap_or(0.0),
            center_y: f.get(2).copied().unwrap_or(0.0),
            outer_diameter: f.get(3).copied().unwrap_or(0.0),
            inner_diameter: f.get(4).copied().unwrap_or(0.0),
            gap: f.get(5).copied().unwrap_or(0.0),
        },
        other => MacroPrimitive::Other { code: other, fields: f[1..].to_vec() },
    })
}

/// Serialize a macro as `%AM{name}*{primitives joined by *}*%`.
pub fn write_am(m: &ApertureMacro) -> String {
    let body = m.primitives.iter().map(|p| p.to_text()).collect::<Vec<_>>().join("*");
    format!("%AM{}*{}{}%", m.name, body, if m.primitives.is_empty() { "" } else { "*" })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_circle_and_rect() {
        let (id, ap) = parse_add("ADD10C,0.5").unwrap();
        assert_eq!(id, 10);
        assert_eq!(ap, Aperture::Circle { diameter: 0.5, hole_diameter: None });
        let (_, r) = parse_add("ADD11R,1.0X2.0").unwrap();
        assert_eq!(r, Aperture::Rectangle { width: 1.0, height: 2.0, hole_diameter: None });
    }

    #[test]
    fn parse_circle_with_hole() {
        let (_, ap) = parse_add("ADD10C,0.5X0.2").unwrap();
        assert_eq!(ap, Aperture::Circle { diameter: 0.5, hole_diameter: Some(0.2) });
        assert_eq!(ap.hole_diameter(), Some(0.2));
        assert_eq!(ap.nominal_width(), Some(0.5));
    }

    #[test]
    fn add_round_trips() {
        for def in ["ADD10C,0.5", "ADD11R,1.0X2.0X0.1", "ADD12O,0.8X1.6", "ADD13P,1.0X5X30.0X0.3"] {
            let (id, ap) = parse_add(def).unwrap();
            let out = write_add(id, &ap);
            let inner = &out[1..out.len() - 2]; // strip leading '%' and trailing '*%'
            let (id2, ap2) = parse_add(inner).unwrap();
            assert_eq!((id, ap), (id2, ap2), "def={def}, out={out}");
        }
    }

    #[test]
    fn macro_round_trips() {
        let src = "AMRECT*4,0,0,0.5,1.0,0.0*4,0,0,0.5,1.0,90.0";
        let m = parse_am(src).unwrap();
        assert_eq!(m.name, "RECT");
        assert_eq!(m.primitives.len(), 2);
        let out = write_am(&m);
        let inner = &out[1..out.len() - 2]; // strip leading '%' and trailing '*%'
        let reparsed = parse_am(inner).unwrap();
        assert_eq!(m, reparsed);
    }
}
