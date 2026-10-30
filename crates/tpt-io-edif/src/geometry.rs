//! EDIF schematic/layout geometric data extraction.
//!
//! SCHEMATIC and LAYOUT views carry geometric figures inside their `(contents …)`
//! as `(figure …)` forms (optionally wrapped in a `(geometry …)` container). This
//! module models those figures — points, rectangles (`rectangle`/`rect`),
//! polylines (`path`), arcs, circles, polygons, line segments, and text labels —
//! and parses them from the generic S-expression tree.
//!
//! The same figure vocabulary is used by both schematic and layout
//! representations; the owning [`View`](crate::model::View)'s
//! [`ViewType`](crate::model::ViewType) (captured on the view) is what distinguishes
//! a symbolic schematic annotation from a physical layout shape. Use
//! [`Edif::geometry_for`](crate::model::Edif::geometry_for) to pull geometry out by
//! representation type.

use alloc::string::String;
use alloc::vec::Vec;

use crate::model::Property;
use crate::parser::Sexp;

/// A 2D point in an EDIF schematic/layout coordinate system.
///
/// Coordinates are resolved from the EDIF `(pt …)` form, which may spell a number
/// literally (`(pt 10 20)`) or wrap each axis in `(integer n)` / `(number n)` /
/// `(real n)` (e.g. `(pt (integer 10) (integer 20))`).
#[derive(Debug, Clone, PartialEq)]
pub struct Point {
    /// X coordinate.
    pub x: f64,
    /// Y coordinate.
    pub y: f64,
}

/// A geometric shape inside a SCHEMATIC or LAYOUT figure.
#[derive(Debug, Clone, PartialEq)]
pub enum Shape {
    /// Axis-aligned bounding-box rectangle, defined by two opposite corners.
    Rectangle {
        /// First corner.
        a: Point,
        /// Opposite corner.
        b: Point,
    },
    /// A polyline of connected points with an optional stroke width.
    Path {
        /// Stroke width, if declared (`(pathWidth …)`).
        width: Option<f64>,
        /// Ordered vertices.
        points: Vec<Point>,
    },
    /// A circle given by its center and diameter (`(diameter …)` or `2*radius`).
    Circle {
        /// Center.
        center: Point,
        /// Diameter.
        diameter: f64,
    },
    /// A closed polygon given as an ordered list of vertices.
    Polygon(Vec<Point>),
    /// A circular arc.
    Arc {
        /// Center.
        center: Point,
        /// Radius.
        radius: f64,
        /// Start angle, in degrees.
        start_angle: f64,
        /// Sweep angle, in degrees (signed).
        sweep_angle: f64,
    },
    /// A straight line segment between two points.
    Line {
        /// First endpoint.
        a: Point,
        /// Second endpoint.
        b: Point,
    },
    /// A text label at a position.
    Label {
        /// Label text.
        text: String,
        /// Anchor position.
        at: Point,
    },
}

/// A parsed geometric figure inside a SCHEMATIC or LAYOUT view's contents.
#[derive(Debug, Clone, PartialEq)]
pub struct Figure {
    /// The figure's shape.
    pub shape: Shape,
    /// Arbitrary properties attached to the figure (`(property key value)`).
    pub properties: Vec<Property>,
}

impl Figure {
    /// Return the figure's shape.
    pub fn shape(&self) -> &Shape {
        &self.shape
    }

    /// Return the figure's properties.
    pub fn properties(&self) -> &[Property] {
        &self.properties
    }
}

/// Parse a single `(figure …)` form into a [`Figure`].
///
/// Returns `None` if the form contains no recognized shape. Unknown property and
/// display sub-forms are tolerated (ignored) so that parse never panics on
/// real-world EDIF that carries extra vendor data.
pub fn read_figure(body: &[Sexp]) -> Option<Figure> {
    let shape = body.iter().find_map(read_shape)?;
    let properties = body
        .iter()
        .filter_map(|f| f.as_form("property"))
        .filter_map(crate::parser::read_property)
        .collect();
    Some(Figure { shape, properties })
}

fn read_shape(form: &Sexp) -> Option<Shape> {
    if let Some(b) = form.as_form("rectangle") {
        let size = b.iter().find_map(|f| f.as_form("rectangleSize"))?;
        let pts: Vec<Point> = size.iter().filter_map(read_point).collect();
        if pts.len() < 2 {
            return None;
        }
        return Some(Shape::Rectangle { a: pts[0].clone(), b: pts[1].clone() });
    }

    // Layout tools frequently emit `(rect …)` (with `rectSize`) for physical shapes.
    if let Some(b) = form.as_form("rect") {
        let size = b.iter().find_map(|f| f.as_form("rectSize"))?;
        let pts: Vec<Point> = size.iter().filter_map(read_point).collect();
        if pts.len() < 2 {
            return None;
        }
        return Some(Shape::Rectangle { a: pts[0].clone(), b: pts[1].clone() });
    }

    if let Some(b) = form.as_form("path") {
        let width =
            b.iter().find_map(|f| f.as_form("pathWidth")).and_then(|w| w.first()).and_then(num_val);
        let points: Vec<Point> = b
            .iter()
            .filter_map(|f| f.as_form("pathPoint"))
            .filter_map(|p| p.iter().find_map(read_point))
            .collect();
        return Some(Shape::Path { width, points });
    }

    if let Some(b) = form.as_form("circle") {
        let size = b.iter().find_map(|f| f.as_form("circleSize"))?;
        let diameter = size
            .iter()
            .find_map(|f| f.as_form("diameter"))
            .and_then(|d| d.first())
            .and_then(num_val)
            .or_else(|| {
                size.iter()
                    .find_map(|f| f.as_form("radius"))
                    .and_then(|r| r.first())
                    .and_then(num_val)
                    .map(|r| r * 2.0)
            })?;
        let center = b.iter().find_map(read_point)?;
        return Some(Shape::Circle { center, diameter });
    }

    if let Some(b) = form.as_form("polygon") {
        let points: Vec<Point> = b.iter().filter_map(read_point).collect();
        if points.is_empty() {
            return None;
        }
        return Some(Shape::Polygon(points));
    }

    if let Some(b) = form.as_form("arc") {
        let radius =
            b.iter().find_map(|f| f.as_form("radius")).and_then(|r| r.first()).and_then(num_val)?;
        let sweep_angle = b
            .iter()
            .find_map(|f| f.as_form("arcAngle"))
            .and_then(|a| a.first())
            .and_then(num_val)
            .unwrap_or(0.0);
        let start_angle = b
            .iter()
            .find_map(|f| f.as_form("angle"))
            .and_then(|a| a.first())
            .and_then(num_val)
            .unwrap_or(0.0);
        let center = b.iter().find_map(read_point)?;
        return Some(Shape::Arc { center, radius, start_angle, sweep_angle });
    }

    if let Some(b) = form.as_form("line") {
        let pts: Vec<Point> = b.iter().filter_map(read_point).collect();
        if pts.len() < 2 {
            return None;
        }
        return Some(Shape::Line { a: pts[0].clone(), b: pts[1].clone() });
    }

    if let Some(b) = form.as_form("label") {
        let text = b
            .iter()
            .find_map(|f| f.as_form("labelText"))
            .and_then(|t| t.first())
            .and_then(|s| match s {
                Sexp::Str(x) => Some(x.clone()),
                Sexp::Sym(x) => Some(x.clone()),
                _ => None,
            })
            .unwrap_or_default();
        let at = b
            .iter()
            .find_map(read_point)
            .or_else(|| {
                b.iter()
                    .find_map(|f| f.as_form("origin"))
                    .and_then(|o| o.iter().find_map(read_point))
            })
            .unwrap_or(Point { x: 0.0, y: 0.0 });
        return Some(Shape::Label { text, at });
    }

    None
}

/// Resolve a `(pt …)` form into a [`Point`].
pub fn read_point(form: &Sexp) -> Option<Point> {
    let tail = form.as_form("pt")?;
    let x = read_coord(tail.first()?)?;
    let y = read_coord(tail.get(1)?)?;
    Some(Point { x, y })
}

/// Read a single coordinate value, which may be a bare number or a wrapped
/// `(integer n)` / `(number n)` / `(real n)` atom.
fn read_coord(s: &Sexp) -> Option<f64> {
    match s {
        Sexp::Num(n) => Some(*n),
        // `(integer 10)` etc.
        Sexp::List(items) => match items.first() {
            Some(Sexp::Sym(k))
                if k.eq_ignore_ascii_case("integer")
                    || k.eq_ignore_ascii_case("number")
                    || k.eq_ignore_ascii_case("real") =>
            {
                match items.get(1) {
                    Some(Sexp::Num(n)) => Some(*n),
                    _ => None,
                }
            }
            _ => None,
        },
        _ => None,
    }
}

/// Read a number value, which may be a bare `(number …)` or a wrapped
/// `(integer n)` / `(number n)` / `(real n)` atom — the same shape EDIF uses for
/// scalar quantities such as `radius`, `diameter`, `pathWidth`, and angles.
fn num_val(s: &Sexp) -> Option<f64> {
    match s {
        Sexp::Num(n) => Some(*n),
        Sexp::List(items) => match items.first() {
            Some(Sexp::Sym(k))
                if k.eq_ignore_ascii_case("integer")
                    || k.eq_ignore_ascii_case("number")
                    || k.eq_ignore_ascii_case("real") =>
            {
                match items.get(1) {
                    Some(Sexp::Num(n)) => Some(*n),
                    _ => None,
                }
            }
            _ => None,
        },
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Cell, Edif, Library, View, ViewType};
    use crate::parser::parse_sexp;
    use crate::writer::serialize;

    fn pt(x: f64, y: f64) -> Point {
        Point { x, y }
    }

    #[test]
    fn parses_rectangle_with_wrapped_coords() {
        let s = parse_sexp(b"(figure (rectangle (rectangleSize (pt (integer 0) (integer 0)) (pt (integer 100) (integer 50)))))")
            .unwrap();
        let fig = read_figure(s[0].as_form("figure").unwrap()).unwrap();
        match &fig.shape {
            Shape::Rectangle { a, b } => {
                assert_eq!(a, &pt(0.0, 0.0));
                assert_eq!(b, &pt(100.0, 50.0));
            }
            other => panic!("expected rectangle, got {other:?}"),
        }
    }

    #[test]
    fn parses_polygon_and_path_and_circle() {
        let poly = parse_sexp(b"(figure (polygon (pt 0 0) (pt 10 0) (pt 5 8)))").unwrap();
        let fig = read_figure(poly[0].as_form("figure").unwrap()).unwrap();
        assert!(matches!(&fig.shape, Shape::Polygon(pts) if pts.len() == 3));

        let path =
            parse_sexp(b"(figure (path (pathWidth 2) (pathPoint (pt 0 0)) (pathPoint (pt 4 4))))")
                .unwrap();
        let fig = read_figure(path[0].as_form("figure").unwrap()).unwrap();
        match &fig.shape {
            Shape::Path { width, points } => {
                assert_eq!(*width, Some(2.0));
                assert_eq!(points.len(), 2);
            }
            other => panic!("expected path, got {other:?}"),
        }

        let circle = parse_sexp(
            b"(figure (circle (circleSize (radius (integer 5))) (pt (integer 10) (integer 10))))",
        )
        .unwrap();
        let fig = read_figure(circle[0].as_form("figure").unwrap()).unwrap();
        match &fig.shape {
            Shape::Circle { center, diameter } => {
                assert_eq!(center, &pt(10.0, 10.0));
                assert_eq!(*diameter, 10.0);
            }
            other => panic!("expected circle, got {other:?}"),
        }
    }

    #[test]
    fn layout_rect_and_schematic_label_round_trip() {
        // Build a document with a SCHEMATIC view (label) and a LAYOUT view (rect),
        // serialize, re-parse, and assert both representations were extracted.
        let schematic = View {
            name: "sym".into(),
            view_type: ViewType::Schematic,
            interface: vec![],
            instances: vec![],
            nets: vec![],
            properties: vec![],
            geometry: vec![Figure {
                shape: Shape::Label { text: "INV".into(), at: pt(1.0, 2.0) },
                properties: vec![],
            }],
        };
        let layout = View {
            name: "phy".into(),
            view_type: ViewType::Layout,
            interface: vec![],
            instances: vec![],
            nets: vec![],
            properties: vec![],
            geometry: vec![Figure {
                shape: Shape::Rectangle { a: pt(0.0, 0.0), b: pt(20.0, 10.0) },
                properties: vec![],
            }],
        };
        let cell = Cell {
            name: "c".into(),
            cell_type: Some("GENERIC".into()),
            views: vec![schematic, layout],
        };
        let edif = Edif {
            name: "top".into(),
            version: (2, 0, 0),
            libraries: vec![Library { name: "lib".into(), cells: vec![cell] }],
            design: None,
        };

        let bytes = serialize(&edif);
        let reparsed = crate::parser::parse_edif(&bytes).unwrap();

        let schematic = reparsed.find_cell("lib", "c").unwrap().views[0].clone();
        let layout = reparsed.find_cell("lib", "c").unwrap().views[1].clone();
        assert_eq!(schematic.view_type, ViewType::Schematic);
        assert!(matches!(&schematic.geometry[0].shape, Shape::Label { text, .. } if text == "INV"));
        assert_eq!(layout.view_type, ViewType::Layout);
        assert!(matches!(&layout.geometry[0].shape, Shape::Rectangle { .. }));

        // `geometry_for` filters by representation type.
        assert_eq!(reparsed.geometry_for(ViewType::Schematic).len(), 1);
        assert_eq!(reparsed.geometry_for(ViewType::Layout).len(), 1);
        assert_eq!(reparsed.geometry_for(ViewType::Netlist).len(), 0);
    }
}
