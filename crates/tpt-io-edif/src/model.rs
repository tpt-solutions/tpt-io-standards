//! EDIF data model.
//!
//! The model mirrors the EDIF 2 0 0 / 4 0 0 hierarchy: an [`Edif`] document holds
//! [`Library`]s, each holding [`Cell`]s, each holding [`View`]s. A NETLIST view
//! carries an [`interface`] of [`Port`]s plus [`contents`] made of [`Instance`]s
//! and [`Net`]s — the pieces needed for netlist extraction and flattening. A
//! SCHEMATIC or LAYOUT view additionally carries [`geometry`](View::geometry) —
//! the visual/physical figures extracted by the [`geometry`](crate::geometry)
//! module.

use alloc::string::{String, ToString};
use alloc::vec::Vec;

use crate::geometry::Figure;

/// The (optional) signal direction of a port.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    /// Input only.
    In,
    /// Output only.
    Out,
    /// Input or output.
    Inout,
    /// Bidirectional.
    Bidirectional,
}

impl Direction {
    /// Parse a direction keyword (case-insensitive). `None` if unknown.
    pub fn parse(s: &str) -> Option<Direction> {
        match s.to_ascii_uppercase().as_str() {
            "IN" => Some(Direction::In),
            "OUT" => Some(Direction::Out),
            "INOUT" => Some(Direction::Inout),
            "BIDIRECTIONAL" => Some(Direction::Bidirectional),
            _ => None,
        }
    }
}

/// The kind of a cell view.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ViewType {
    /// Netlist (connectivity) view.
    Netlist,
    /// Schematic (geometric) view.
    Schematic,
    /// Physical layout view.
    Layout,
    /// Symbol view.
    Symbol,
    /// A view type not enumerated here (e.g. `DOCUMENT`, `PCB`).
    Other(String),
}

impl ViewType {
    /// Parse a view type keyword (case-insensitive).
    pub fn parse(s: &str) -> ViewType {
        match s.to_ascii_uppercase().as_str() {
            "NETLIST" => ViewType::Netlist,
            "SCHEMATIC" => ViewType::Schematic,
            "LAYOUT" => ViewType::Layout,
            "SYMBOL" => ViewType::Symbol,
            other => ViewType::Other(other.to_string()),
        }
    }
}

/// A signal port on a cell interface.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Port {
    /// Effective port name (after any `rename`).
    pub name: String,
    /// Signal direction, if declared.
    pub direction: Option<Direction>,
}

/// A reference to a port, optionally scoped to an instance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PortRef {
    /// Port name.
    pub port: String,
    /// Instance the port belongs to (`None` for a top-level interface port).
    pub instance: Option<String>,
}

/// A net connecting one or more port references.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Net {
    /// Effective net name (after any `rename`).
    pub name: String,
    /// Connected ports.
    pub members: Vec<PortRef>,
}

/// A cell instance inside a view's contents.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Instance {
    /// Instance name.
    pub name: String,
    /// Referenced cell name.
    pub cell_ref: String,
    /// Referenced view name (often `NETLIST`).
    pub view_ref: String,
    /// Referenced library name.
    pub library_ref: String,
}

/// A parsed cell view.
#[derive(Debug, Clone, PartialEq)]
pub struct View {
    /// View name.
    pub name: String,
    /// View type.
    pub view_type: ViewType,
    /// Interface ports.
    pub interface: Vec<Port>,
    /// Instances declared in the view contents.
    pub instances: Vec<Instance>,
    /// Nets declared in the view contents.
    pub nets: Vec<Net>,
    /// Arbitrary properties attached to the view (`(property key (owner ...) value)`).
    pub properties: Vec<Property>,
    /// Geometric figures extracted from a SCHEMATIC/LAYOUT view's contents.
    pub geometry: Vec<Figure>,
}

/// A key/value property.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Property {
    /// Property key.
    pub key: String,
    /// Property value (string or number).
    pub value: String,
}

/// A parsed cell.
#[derive(Debug, Clone, PartialEq)]
pub struct Cell {
    /// Cell name.
    pub name: String,
    /// Cell type keyword (`GENERIC`, `SOFTWARE`, …), if declared.
    pub cell_type: Option<String>,
    /// Views contained in this cell.
    pub views: Vec<View>,
}

impl Cell {
    /// Return the first NETLIST view, if any.
    pub fn netlist_view(&self) -> Option<&View> {
        self.views.iter().find(|v| v.view_type == ViewType::Netlist)
    }

    /// Return the first SCHEMATIC view, if any.
    pub fn schematic_view(&self) -> Option<&View> {
        self.views.iter().find(|v| v.view_type == ViewType::Schematic)
    }

    /// Return the first LAYOUT view, if any.
    pub fn layout_view(&self) -> Option<&View> {
        self.views.iter().find(|v| v.view_type == ViewType::Layout)
    }
}

/// A parsed library.
#[derive(Debug, Clone, PartialEq)]
pub struct Library {
    /// Library name.
    pub name: String,
    /// Cells contained in this library.
    pub cells: Vec<Cell>,
}

impl Library {
    /// Look up a cell by name.
    pub fn cell(&self, name: &str) -> Option<&Cell> {
        self.cells.iter().find(|c| c.name == name)
    }
}

/// A parsed design (top-level instantiation).
#[derive(Debug, Clone, PartialEq)]
pub struct Design {
    /// Design name.
    pub name: String,
    /// Top cell reference.
    pub cell_ref: String,
    /// Top library reference.
    pub library_ref: String,
}

/// A fully parsed EDIF document.
#[derive(Debug, Clone, PartialEq)]
pub struct Edif {
    /// Top-level name (first arg of `(edif …)`).
    pub name: String,
    /// EDIF version tuple, e.g. `(2, 0, 0)`.
    pub version: (u32, u32, u32),
    /// Libraries declared in the document.
    pub libraries: Vec<Library>,
    /// The design, if present.
    pub design: Option<Design>,
}

impl View {
    /// Return the geometric figures carried by this view (empty for NETLIST views
    /// that have no drawing data).
    pub fn geometry(&self) -> &[Figure] {
        &self.geometry
    }
}

impl Edif {
    /// Look up a library by name.
    pub fn library(&self, name: &str) -> Option<&Library> {
        self.libraries.iter().find(|l| l.name == name)
    }

    /// Find a cell by name across all libraries.
    pub fn find_cell(&self, library: &str, cell: &str) -> Option<&Cell> {
        self.library(library).and_then(|l| l.cell(cell))
    }

    /// Collect every geometric figure from every view whose type matches `vt`.
    ///
    /// This is the entry point for schematic (`ViewType::Schematic`) and layout
    /// (`ViewType::Layout`) geometric data extraction: pass the representation
    /// type you want and receive all figures across the whole document.
    pub fn geometry_for(&self, vt: ViewType) -> Vec<&Figure> {
        let mut out = Vec::new();
        for lib in &self.libraries {
            for cell in &lib.cells {
                for view in &cell.views {
                    if view.view_type == vt {
                        for f in &view.geometry {
                            out.push(f);
                        }
                    }
                }
            }
        }
        out
    }
}
