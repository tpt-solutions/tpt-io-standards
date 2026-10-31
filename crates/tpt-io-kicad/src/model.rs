//! Typed view of a KiCad PCB, schematic or symbol library.
//!
//! The reader targets the KiCad 6 and later S-expression layout: a top-level
//! `kicad_pcb`, `kicad_sch` or `kicad_symbol_lib` list with `version` and
//! `generator` entries. PCB files carry nets, footprints (with pads), tracks
//! (`segment`) and vias. Schematics carry placed symbols and wires. Symbol
//! libraries list their symbols.

use alloc::string::String;
use alloc::vec::Vec;

use crate::error::KicadError;
use crate::tree::{parse_tree, Node};

/// The kind of KiCad document.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocKind {
    /// A PCB layout (`.kicad_pcb`).
    Pcb,
    /// A schematic (`.kicad_sch`).
    Schematic,
    /// A symbol library (`.kicad_sym`).
    SymbolLib,
}

/// A net declared in a PCB file.
#[derive(Debug, Clone, PartialEq)]
pub struct Net {
    /// The net number.
    pub id: u32,
    /// The net name; empty for the unconnected net 0.
    pub name: String,
}

/// A pad of a footprint.
#[derive(Debug, Clone, PartialEq)]
pub struct Pad {
    /// The pad number or name, e.g. `1`.
    pub number: String,
    /// `smd`, `thru_hole`, `np_thru_hole` or `connect`.
    pub kind: String,
    /// `rect`, `roundrect`, `circle`, `oval`, …
    pub shape: String,
    /// Position relative to the footprint origin, in millimetres.
    pub x: f64,
    /// Position relative to the footprint origin, in millimetres.
    pub y: f64,
    /// Width in millimetres.
    pub width: f64,
    /// Height in millimetres.
    pub height: f64,
    /// The net the pad is connected to, if any.
    pub net: Option<u32>,
}

/// A placed footprint in a PCB file.
#[derive(Debug, Clone, PartialEq)]
pub struct Footprint {
    /// The library identifier, `library:footprint`.
    pub lib_id: String,
    /// The copper or silkscreen layer, e.g. `F.Cu`.
    pub layer: Option<String>,
    /// Position in millimetres.
    pub x: f64,
    /// Position in millimetres.
    pub y: f64,
    /// Rotation in degrees (0 when absent).
    pub rotation: f64,
    /// The `Reference` property, e.g. `R1`.
    pub reference: Option<String>,
    /// The `Value` property, e.g. `10k`.
    pub value: Option<String>,
    /// The pads of the footprint.
    pub pads: Vec<Pad>,
}

/// A copper track segment in a PCB file.
#[derive(Debug, Clone, PartialEq)]
pub struct Track {
    /// Start point `(x, y)` in millimetres.
    pub start: (f64, f64),
    /// End point `(x, y)` in millimetres.
    pub end: (f64, f64),
    /// Width in millimetres.
    pub width: f64,
    /// The copper layer, e.g. `F.Cu`.
    pub layer: String,
    /// The net, if any.
    pub net: Option<u32>,
}

/// A via in a PCB file.
#[derive(Debug, Clone, PartialEq)]
pub struct Via {
    /// Position `(x, y)` in millimetres.
    pub at: (f64, f64),
    /// Pad diameter in millimetres.
    pub size: f64,
    /// Drill diameter in millimetres.
    pub drill: f64,
    /// The copper layers the via connects.
    pub layers: Vec<String>,
    /// The net, if any.
    pub net: Option<u32>,
}

/// A placed symbol in a schematic.
#[derive(Debug, Clone, PartialEq)]
pub struct Symbol {
    /// The library identifier, `library:symbol`.
    pub lib_id: String,
    /// Position in millimetres.
    pub x: f64,
    /// Position in millimetres.
    pub y: f64,
    /// Rotation in degrees (0 when absent).
    pub rotation: f64,
    /// The `Reference` property, e.g. `R1`.
    pub reference: Option<String>,
    /// The `Value` property, e.g. `10k`.
    pub value: Option<String>,
}

/// A schematic wire: a polyline of points.
#[derive(Debug, Clone, PartialEq)]
pub struct Wire {
    /// The points of the polyline, in millimetres.
    pub points: Vec<(f64, f64)>,
}

/// A symbol defined in a symbol library.
#[derive(Debug, Clone, PartialEq)]
pub struct LibSymbol {
    /// The symbol name, e.g. `R` or `Device:R` in a library.
    pub name: String,
    /// The default `Reference` property, e.g. `R`.
    pub reference: Option<String>,
    /// The default `Value` property.
    pub value: Option<String>,
}

/// The kind of a schematic label. Local labels apply to one sheet; global and hierarchical labels connect sheets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LabelKind {
    /// A `label`: a net name on one sheet.
    Local,
    /// A `global_label`: a net name shared by every sheet.
    Global,
    /// A `hierarchical_label`: a net name that connects to a sheet pin.
    Hierarchical,
}

/// A schematic label: the net name placed on a wire.
#[derive(Debug, Clone, PartialEq)]
pub struct Label {
    /// Which kind of label this is.
    pub kind: LabelKind,
    /// The label text (the net name).
    pub text: String,
    /// The label position.
    pub at: (f64, f64),
    /// The rotation in degrees.
    pub rotation: f64,
}

/// A schematic junction: the dot drawn where wires join.
#[derive(Debug, Clone, PartialEq)]
pub struct Junction {
    /// The junction position.
    pub at: (f64, f64),
    /// The dot diameter; 0 means the default size.
    pub diameter: f64,
}

/// A PCB copper zone (pour): its net, layer, outline, and the filled areas KiCad computed for it.
#[derive(Debug, Clone, PartialEq)]
pub struct Zone {
    /// The net number, if the zone is connected to one.
    pub net: Option<u32>,
    /// The net name, if given.
    pub net_name: Option<String>,
    /// The layer the zone is on.
    pub layer: Option<String>,
    /// The zone outline, from its `polygon` child.
    pub outline: Vec<(f64, f64)>,
    /// The filled areas, from its `filled_polygon` children, each a list of points.
    pub fills: Vec<Vec<(f64, f64)>>,
}

/// A parsed KiCad document.
#[derive(Debug, Clone, PartialEq)]
pub struct KicadDocument {
    /// PCB, schematic or symbol library.
    pub kind: DocKind,
    /// The file-format version number, e.g. `20211014`.
    pub version: Option<u32>,
    /// The program that wrote the file, e.g. `pcbnew`.
    pub generator: Option<String>,
    /// Nets (PCB only).
    pub nets: Vec<Net>,
    /// Footprints (PCB only).
    pub footprints: Vec<Footprint>,
    /// Copper track segments (PCB only).
    pub tracks: Vec<Track>,
    /// Vias (PCB only).
    pub vias: Vec<Via>,
    /// Placed symbols (schematic only).
    pub symbols: Vec<Symbol>,
    /// Wires (schematic only).
    pub wires: Vec<Wire>,
    /// Symbols defined in a symbol library (library only).
    pub lib_symbols: Vec<LibSymbol>,
    /// Copper zones (PCB only): outlines and the filled areas KiCad computed.
    pub zones: Vec<Zone>,
    /// Schematic labels (schematic only): local, global and hierarchical.
    pub labels: Vec<Label>,
    /// Schematic junctions (schematic only): dots where wires join.
    pub junctions: Vec<Junction>,
}

/// Parse a `.kicad_pcb`, `.kicad_sch` or `.kicad_sym` file.
pub fn parse(input: &[u8]) -> Result<KicadDocument, KicadError> {
    let root = parse_tree(input)?;
    let kind = match root.head() {
        Some("kicad_pcb") => DocKind::Pcb,
        Some("kicad_sch") => DocKind::Schematic,
        Some("kicad_symbol_lib") => DocKind::SymbolLib,
        _ => return Err(KicadError::Unsupported("not a kicad_pcb, kicad_sch or kicad_sym file")),
    };

    let version = root.find("version").and_then(|n| n.arg_f64(0)).map(|v| v as u32);
    // KiCad 5 writes `(host pcbnew …)` where KiCad 6 writes `(generator pcbnew)`.
    let generator = root
        .find("generator")
        .or_else(|| root.find("host"))
        .and_then(|n| n.arg_str(0))
        .map(Into::into);

    let mut doc = KicadDocument {
        kind,
        version,
        generator,
        nets: Vec::new(),
        footprints: Vec::new(),
        tracks: Vec::new(),
        vias: Vec::new(),
        symbols: Vec::new(),
        wires: Vec::new(),
        lib_symbols: Vec::new(),
        zones: Vec::new(),
        labels: Vec::new(),
        junctions: Vec::new(),
    };

    match kind {
        DocKind::Pcb => {
            for n in root.children("net") {
                let id = n.arg_f64(0).unwrap_or(0.0) as u32;
                let name = n.arg_str(1).unwrap_or("").into();
                doc.nets.push(Net { id, name });
            }
            // KiCad 5 calls footprints `module`; the body is otherwise the same.
            for n in root.children("footprint").chain(root.children("module")) {
                doc.footprints.push(footprint(n));
            }
            for n in root.children("segment") {
                doc.tracks.push(track(n));
            }
            for n in root.children("via") {
                doc.vias.push(via(n));
            }
            for n in root.children("zone") {
                doc.zones.push(zone(n));
            }
        }
        DocKind::Schematic => {
            for n in root.children("symbol") {
                doc.symbols.push(symbol(n));
            }
            for n in root.children("wire") {
                doc.wires.push(wire(n));
            }
            for n in root.children("label") {
                doc.labels.push(label(n, LabelKind::Local));
            }
            for n in root.children("global_label") {
                doc.labels.push(label(n, LabelKind::Global));
            }
            for n in root.children("hierarchical_label") {
                doc.labels.push(label(n, LabelKind::Hierarchical));
            }
            for n in root.children("junction") {
                doc.junctions.push(junction(n));
            }
        }
        DocKind::SymbolLib => {
            for n in root.children("symbol") {
                doc.lib_symbols.push(LibSymbol {
                    name: n.arg_str(0).unwrap_or("").into(),
                    reference: property(n, "Reference"),
                    value: property(n, "Value"),
                });
            }
        }
    }
    Ok(doc)
}

fn label(node: &Node, kind: LabelKind) -> Label {
    let (x, y, rotation) = placement(node);
    Label { kind, text: node.arg_str(0).unwrap_or("").into(), at: (x, y), rotation }
}

fn junction(node: &Node) -> Junction {
    let (x, y, _) = placement(node);
    Junction {
        at: (x, y),
        diameter: node.find("diameter").and_then(|d| d.arg_f64(0)).unwrap_or(0.0),
    }
}

fn zone(node: &Node) -> Zone {
    Zone {
        net: node.find("net").and_then(|n| n.arg_f64(0)).map(|v| v as u32),
        net_name: node.find("net_name").and_then(|n| n.arg_str(0)).map(Into::into),
        layer: node.find("layer").and_then(|n| n.arg_str(0)).map(Into::into),
        outline: node.find("polygon").map(points).unwrap_or_default(),
        fills: node.children("filled_polygon").map(points).collect(),
    }
}

/// The points of the `(pts (xy x y) …)` child of `node`.
fn points(node: &Node) -> Vec<(f64, f64)> {
    node.find("pts")
        .map(|pts| pts.children("xy").map(|xy| point(Some(xy))).collect())
        .unwrap_or_default()
}

/// The value of the `(property "name" "value" …)` child named `name`.
fn property(node: &Node, name: &str) -> Option<String> {
    node.children("property")
        .find(|p| p.arg_str(0) == Some(name))
        .and_then(|p| p.arg_str(1))
        .map(Into::into)
}

/// The text of a KiCad 5 `(fp_text kind "text" …)` child, e.g. `reference` or `value`.
fn fp_text(node: &Node, kind: &str) -> Option<String> {
    node.children("fp_text")
        .find(|t| t.arg_str(0) == Some(kind))
        .and_then(|t| t.arg_str(1))
        .map(Into::into)
}

/// The `(x y)` pair at the start of a node's arguments, e.g. from `(start x y)`.
fn point(node: Option<&Node>) -> (f64, f64) {
    match node {
        Some(n) => (n.arg_f64(0).unwrap_or(0.0), n.arg_f64(1).unwrap_or(0.0)),
        None => (0.0, 0.0),
    }
}

/// Position and rotation from `(at x y [rotation])`.
fn placement(node: &Node) -> (f64, f64, f64) {
    let (x, y) = point(node.find("at"));
    let rotation = node.find("at").and_then(|at| at.arg_f64(2)).unwrap_or(0.0);
    (x, y, rotation)
}

/// The net number from `(net id …)`.
fn net_of(node: &Node) -> Option<u32> {
    node.find("net").and_then(|n| n.arg_f64(0)).map(|v| v as u32)
}

fn footprint(node: &Node) -> Footprint {
    let (x, y, rotation) = placement(node);
    Footprint {
        lib_id: node.arg_str(0).unwrap_or("").into(),
        layer: node.find("layer").and_then(|l| l.arg_str(0)).map(Into::into),
        x,
        y,
        rotation,
        reference: property(node, "Reference").or_else(|| fp_text(node, "reference")),
        value: property(node, "Value").or_else(|| fp_text(node, "value")),
        pads: node.children("pad").map(pad).collect(),
    }
}

fn pad(node: &Node) -> Pad {
    let (x, y) = point(node.find("at"));
    let (width, height) = point(node.find("size"));
    // KiCad 5 writes pad numbers unquoted (`(pad 1 smd …)`), so accept numbers too.
    let number = match node.arg(0) {
        Some(Node::Number(n)) => format!("{}", n),
        _ => node.arg_str(0).unwrap_or("").into(),
    };
    Pad {
        number,
        kind: node.arg_str(1).unwrap_or("").into(),
        shape: node.arg_str(2).unwrap_or("").into(),
        x,
        y,
        width,
        height,
        net: net_of(node),
    }
}

fn track(node: &Node) -> Track {
    Track {
        start: point(node.find("start")),
        end: point(node.find("end")),
        width: node.find("width").and_then(|w| w.arg_f64(0)).unwrap_or(0.0),
        layer: node.find("layer").and_then(|l| l.arg_str(0)).unwrap_or("").into(),
        net: net_of(node),
    }
}

fn via(node: &Node) -> Via {
    let (x, y) = point(node.find("at"));
    Via {
        at: (x, y),
        size: node.find("size").and_then(|s| s.arg_f64(0)).unwrap_or(0.0),
        drill: node.find("drill").and_then(|d| d.arg_f64(0)).unwrap_or(0.0),
        layers: node
            .find("layers")
            .map(|l| {
                l.items()
                    .iter()
                    .skip(1)
                    .filter_map(|i| match i {
                        crate::tree::Node::Str(s) | crate::tree::Node::Symbol(s) => Some(s.clone()),
                        _ => None,
                    })
                    .collect()
            })
            .unwrap_or_default(),
        net: net_of(node),
    }
}

fn symbol(node: &Node) -> Symbol {
    let (x, y, rotation) = placement(node);
    Symbol {
        lib_id: node.find("lib_id").and_then(|l| l.arg_str(0)).unwrap_or("").into(),
        x,
        y,
        rotation,
        reference: property(node, "Reference"),
        value: property(node, "Value"),
    }
}

fn wire(node: &Node) -> Wire {
    let points = node
        .find("pts")
        .map(|pts| pts.children("xy").map(|xy| point(Some(xy))).collect())
        .unwrap_or_default();
    Wire { points }
}
