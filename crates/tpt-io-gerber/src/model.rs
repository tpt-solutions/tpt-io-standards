//! Strongly-typed data model for a parsed Gerber (RS-274X / X2 / X3) file.

use alloc::vec::Vec;

use crate::aperture::{Aperture, ApertureMacro};
use crate::attributes::Attributes;
use crate::coord::CoordinateFormat;

/// Linear-unit system of the file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Unit {
    /// Imperial inches.
    Inch,
    /// Metric millimetres.
    #[default]
    Mm,
}

impl Unit {
    /// The Gerber `MO` code letter (`IN` or `MM`).
    pub fn mo_code(&self) -> &'static str {
        match self {
            Unit::Inch => "IN",
            Unit::Mm => "MM",
        }
    }
}

/// Layer polarity (`%LP…%` in X1, `%TF.FilePolarity…%` in X2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Polarity {
    /// Drawn features add to the image.
    #[default]
    Positive,
    /// Drawn features subtract from the image.
    Negative,
}

/// Interpolation mode between consecutive points.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Interpolation {
    /// Straight-line (linear) interpolation.
    #[default]
    Linear,
    /// Clockwise circular interpolation.
    CircularCw,
    /// Counter-clockwise circular interpolation.
    CircularCcw,
}

/// Quadrant mode for circular interpolation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Quadrant {
    /// Single-quadrant arcs.
    Single,
    /// Multi-quadrant arcs.
    #[default]
    Multi,
}

/// A graphic primitive (after aperture/coordinate resolution).
#[derive(Debug, Clone, PartialEq)]
pub enum ObjectKind {
    /// A copper flash (D03) at `(x, y)` using the current aperture.
    Flash {
        /// X position.
        x: f64,
        /// Y position.
        y: f64,
    },
    /// A straight trace segment from `(x0, y0)` to `(x1, y1)`.
    Line {
        /// Segment start X.
        x0: f64,
        /// Segment start Y.
        y0: f64,
        /// Segment end X.
        x1: f64,
        /// Segment end Y.
        y1: f64,
    },
    /// A circular arc from `(x0, y0)` to `(x1, y1)` with center offset `(i, j)`.
    Arc {
        /// Arc start X.
        x0: f64,
        /// Arc start Y.
        y0: f64,
        /// Arc end X.
        x1: f64,
        /// Arc end Y.
        y1: f64,
        /// Center X offset from the start point.
        i: f64,
        /// Center Y offset from the start point.
        j: f64,
        /// Direction: `true` = counter-clockwise.
        ccw: bool,
    },
    /// A filled polygon region (between `G36` … `G37`).
    Region {
        /// Ordered vertices; the region is implicitly closed.
        points: Vec<(f64, f64)>,
    },
}

/// A single graphic object together with its aperture and attribute context.
#[derive(Debug, Clone, PartialEq)]
pub struct GraphicObject {
    /// The primitive itself.
    pub kind: ObjectKind,
    /// Aperture used (for flashes, lines, arcs). `None` for regions.
    pub aperture: Option<u32>,
    /// Object-level (X3) attributes, e.g. net / component / pin.
    pub attributes: Attributes,
}

/// A fully parsed Gerber file.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct GerberFile {
    /// Coordinate format (from the `FS` statement).
    pub format: CoordinateFormat,
    /// Linear unit (from the `MO` statement; defaults to mm).
    pub unit: Unit,
    /// Layer polarity.
    pub polarity: Polarity,
    /// File-level (`%TF…%`) attributes.
    pub file_attributes: Attributes,
    /// Defined standard/macro apertures keyed by aperture number.
    pub apertures: Vec<(u32, Aperture)>,
    /// Aperture-level (`%TA…%`) attributes keyed by aperture number.
    pub aperture_attributes: Vec<(u32, Attributes)>,
    /// Defined aperture macros keyed by name.
    pub aperture_macros: Vec<(String, ApertureMacro)>,
    /// Graphic objects in draw order.
    pub objects: Vec<GraphicObject>,
    /// Whether an `M02`/`M00` end-of-file was encountered.
    pub end_of_file: bool,
}
