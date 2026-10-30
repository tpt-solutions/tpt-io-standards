//! Typed view of a KiCad PCB or schematic.
//!
//! The reader targets the KiCad 6 and later S-expression layout: a top-level
//! `kicad_pcb` or `kicad_sch` list with `version` and `generator` entries, nets
//! in `(net id "name")` form, footprints as `(footprint "lib:name" …)`, and
//! placed schematic symbols as `(symbol (lib_id "…") …)`.

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
}

/// A net declared in a PCB file.
#[derive(Debug, Clone, PartialEq)]
pub struct Net {
    /// The net number.
    pub id: u32,
    /// The net name; empty for the unconnected net 0.
    pub name: String,
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

/// A parsed KiCad document.
#[derive(Debug, Clone, PartialEq)]
pub struct KicadDocument {
    /// PCB or schematic.
    pub kind: DocKind,
    /// The file-format version number, e.g. `20211014`.
    pub version: Option<u32>,
    /// The program that wrote the file, e.g. `pcbnew`.
    pub generator: Option<String>,
    /// Nets (PCB only; empty for schematics).
    pub nets: Vec<Net>,
    /// Footprints (PCB only).
    pub footprints: Vec<Footprint>,
    /// Placed symbols (schematic only).
    pub symbols: Vec<Symbol>,
}

/// Parse a `.kicad_pcb` or `.kicad_sch` file.
pub fn parse(input: &[u8]) -> Result<KicadDocument, KicadError> {
    let root = parse_tree(input)?;
    let kind = match root.head() {
        Some("kicad_pcb") => DocKind::Pcb,
        Some("kicad_sch") => DocKind::Schematic,
        _ => return Err(KicadError::Unsupported("not a kicad_pcb or kicad_sch file")),
    };

    let version = root.find("version").and_then(|n| n.arg_f64(0)).map(|v| v as u32);
    let generator = root.find("generator").and_then(|n| n.arg_str(0)).map(Into::into);

    let mut doc = KicadDocument {
        kind,
        version,
        generator,
        nets: Vec::new(),
        footprints: Vec::new(),
        symbols: Vec::new(),
    };

    match kind {
        DocKind::Pcb => {
            for n in root.children("net") {
                let id = n.arg_f64(0).unwrap_or(0.0) as u32;
                let name = n.arg_str(1).unwrap_or("").into();
                doc.nets.push(Net { id, name });
            }
            for n in root.children("footprint") {
                doc.footprints.push(footprint(n));
            }
        }
        DocKind::Schematic => {
            for n in root.children("symbol") {
                doc.symbols.push(symbol(n));
            }
        }
    }
    Ok(doc)
}

/// The value of the `(property "name" "value" …)` child named `name`.
fn property(node: &Node, name: &str) -> Option<String> {
    node.children("property")
        .find(|p| p.arg_str(0) == Some(name))
        .and_then(|p| p.arg_str(1))
        .map(Into::into)
}

/// Position and rotation from `(at x y [rotation])`.
fn placement(node: &Node) -> (f64, f64, f64) {
    match node.find("at") {
        Some(at) => (
            at.arg_f64(0).unwrap_or(0.0),
            at.arg_f64(1).unwrap_or(0.0),
            at.arg_f64(2).unwrap_or(0.0),
        ),
        None => (0.0, 0.0, 0.0),
    }
}

fn footprint(node: &Node) -> Footprint {
    let (x, y, rotation) = placement(node);
    Footprint {
        lib_id: node.arg_str(0).unwrap_or("").into(),
        layer: node.find("layer").and_then(|l| l.arg_str(0)).map(Into::into),
        x,
        y,
        rotation,
        reference: property(node, "Reference"),
        value: property(node, "Value"),
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
