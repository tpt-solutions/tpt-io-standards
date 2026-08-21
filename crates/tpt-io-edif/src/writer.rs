//! EDIF serialization (round-trip).
//!
//! [`serialize`] reconstructs an EDIF document from the typed [`model::Edif`]
//! model. The output is a valid, reparseable EDIF file (semantically equivalent
//! to the input, modulo constructs the model does not retain such as `status` or
//! `keywordMap`). This powers the `parse → serialize → parse` round-trip test.

use alloc::string::String;
use alloc::vec::Vec;

use crate::model::{
    Cell, Design, Direction, Edif, Instance, Library, Net, Port, PortRef, Property, View, ViewType,
};
use crate::parser::Sexp;

impl Edif {
    /// Serialize to an EDIF S-expression document.
    pub fn to_bytes(&self) -> Vec<u8> {
        serialize(self)
    }
}

/// Serialize an [`Edif`] document to EDIF text bytes.
pub fn serialize(edif: &Edif) -> Vec<u8> {
    let sexp = to_sexp(edif);
    let mut out = String::new();
    write_sexp(&sexp, 0, &mut out);
    out.into_bytes()
}

fn to_sexp(edif: &Edif) -> Sexp {
    let mut items = vec![
        Sexp::Sym("edif".into()),
        Sexp::Sym(edif.name.clone()),
        Sexp::List(vec![
            Sexp::Sym("edifVersion".into()),
            Sexp::Num(edif.version.0 as f64),
            Sexp::Num(edif.version.1 as f64),
            Sexp::Num(edif.version.2 as f64),
        ]),
        Sexp::List(vec![Sexp::Sym("edifLevel".into()), Sexp::Num(0.0)]),
    ];
    for lib in &edif.libraries {
        items.push(lib_to_sexp(lib));
    }
    if let Some(d) = &edif.design {
        items.push(design_to_sexp(d));
    }
    Sexp::List(items)
}

fn lib_to_sexp(lib: &Library) -> Sexp {
    let mut items = vec![Sexp::Sym("library".into()), Sexp::Sym(lib.name.clone())];
    for cell in &lib.cells {
        items.push(cell_to_sexp(cell));
    }
    Sexp::List(items)
}

fn cell_to_sexp(cell: &Cell) -> Sexp {
    let mut items = vec![Sexp::Sym("cell".into()), Sexp::Sym(cell.name.clone())];
    if let Some(ct) = &cell.cell_type {
        items.push(Sexp::List(vec![Sexp::Sym("cellType".into()), Sexp::Sym(ct.clone())]));
    }
    for view in &cell.views {
        items.push(view_to_sexp(view));
    }
    Sexp::List(items)
}

fn view_to_sexp(view: &View) -> Sexp {
    let mut items = vec![
        Sexp::Sym("view".into()),
        Sexp::Sym(view.name.clone()),
        Sexp::List(vec![Sexp::Sym("viewType".into()), Sexp::Sym(view_type_sym(&view.view_type))]),
    ];
    if !view.interface.is_empty() {
        let ports: Vec<Sexp> = view.interface.iter().map(port_to_sexp).collect();
        items.push(Sexp::List(alloc::vec![Sexp::Sym("interface".into()); 1].into_iter().chain(ports).collect()));
    }
    let mut contents = Vec::new();
    for inst in &view.instances {
        contents.push(instance_to_sexp(inst));
    }
    for net in &view.nets {
        contents.push(net_to_sexp(net));
    }
    for prop in &view.properties {
        contents.push(prop_to_sexp(prop));
    }
    items.push(Sexp::List(alloc::vec![Sexp::Sym("contents".into())].into_iter().chain(contents).collect()));
    Sexp::List(items)
}

fn view_type_sym(v: &ViewType) -> String {
    match v {
        ViewType::Netlist => "NETLIST".into(),
        ViewType::Schematic => "SCHEMATIC".into(),
        ViewType::Layout => "LAYOUT".into(),
        ViewType::Symbol => "SYMBOL".into(),
        ViewType::Other(s) => s.clone(),
    }
}

fn port_to_sexp(p: &Port) -> Sexp {
    let mut items = vec![Sexp::Sym("port".into()), Sexp::Sym(p.name.clone())];
    if let Some(d) = p.direction {
        items.push(Sexp::List(vec![Sexp::Sym("direction".into()), Sexp::Sym(dir_sym(d))]));
    }
    Sexp::List(items)
}

fn dir_sym(d: Direction) -> String {
    match d {
        Direction::In => "IN".into(),
        Direction::Out => "OUT".into(),
        Direction::Inout => "INOUT".into(),
        Direction::Bidirectional => "BIDIRECTIONAL".into(),
    }
}

fn instance_to_sexp(i: &Instance) -> Sexp {
    Sexp::List(vec![
        Sexp::Sym("instance".into()),
        Sexp::Sym(i.name.clone()),
        Sexp::List(vec![Sexp::Sym("viewRef".into()), Sexp::Sym(i.view_ref.clone())]),
        Sexp::List(vec![Sexp::Sym("cellRef".into()), Sexp::Sym(i.cell_ref.clone())]),
        Sexp::List(vec![Sexp::Sym("libraryRef".into()), Sexp::Sym(i.library_ref.clone())]),
    ])
}

fn net_to_sexp(n: &Net) -> Sexp {
    let joined: Vec<Sexp> = n.members.iter().map(port_ref_to_sexp).collect();
    Sexp::List(vec![
        Sexp::Sym("net".into()),
        Sexp::Sym(n.name.clone()),
        Sexp::List(alloc::vec![Sexp::Sym("joined".into())].into_iter().chain(joined).collect()),
    ])
}

fn port_ref_to_sexp(pr: &PortRef) -> Sexp {
    if let Some(inst) = &pr.instance {
        Sexp::List(vec![
            Sexp::Sym("portRef".into()),
            Sexp::Sym(pr.port.clone()),
            Sexp::List(vec![Sexp::Sym("instanceRef".into()), Sexp::Sym(inst.clone())]),
        ])
    } else {
        Sexp::List(vec![Sexp::Sym("portRef".into()), Sexp::Sym(pr.port.clone())])
    }
}

fn prop_to_sexp(p: &Property) -> Sexp {
    Sexp::List(vec![
        Sexp::Sym("property".into()),
        Sexp::Sym(p.key.clone()),
        Sexp::Str(p.value.clone()),
    ])
}

fn design_to_sexp(d: &Design) -> Sexp {
    Sexp::List(vec![
        Sexp::Sym("design".into()),
        Sexp::Sym(d.name.clone()),
        Sexp::List(vec![Sexp::Sym("cellRef".into()), Sexp::Sym(d.cell_ref.clone())]),
        Sexp::List(vec![Sexp::Sym("libraryRef".into()), Sexp::Sym(d.library_ref.clone())]),
    ])
}

fn write_sexp(s: &Sexp, indent: usize, out: &mut String) {
    match s {
        Sexp::Sym(x) => out.push_str(x),
        Sexp::Num(n) => {
            if *n == n.trunc() && n.abs() < 1e15 {
                out.push_str(&alloc::format!("{}", *n as i64));
            } else {
                out.push_str(&alloc::format!("{n}"));
            }
        }
        Sexp::Str(x) => out.push_str(&alloc::format!("\"{}\"", x.replace('\\', "\\\\").replace('"', "\\\""))),
        Sexp::List(items) => {
            if items.is_empty() {
                out.push_str("()");
                return;
            }
            out.push('(');
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    if matches!(item, Sexp::List(_)) {
                        out.push('\n');
                        out.push_str(&"  ".repeat(indent + 1));
                    } else {
                        out.push(' ');
                    }
                }
                write_sexp(item, indent + 1, out);
            }
            out.push(')');
        }
    }
}

/// A [`tpt_io_core::Writer`] for [`Edif`].
#[derive(Debug, Clone, Default)]
pub struct EdifWriter;

impl EdifWriter {
    /// Create a new writer.
    pub fn new() -> Self {
        Self
    }
}

impl tpt_io_core::Writer<Edif> for EdifWriter {

    type Error = core::convert::Infallible;

    fn serialize(&self, value: &Edif) -> core::result::Result<Vec<u8>, core::convert::Infallible> {
        Ok(serialize(value))
    }
}
