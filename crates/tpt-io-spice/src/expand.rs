//! Hierarchical expansion of `.subckt` instances (`X` cards).
//!
//! SPICE subcircuits are referenced by `X` instances that bind the subcircuit's
//! declared ports to external nodes and pass parameters (`X1 a b inv W=1k`).
//! [`expand_subckt_instance`] instantiates a subcircuit body into concrete
//! element cards:
//!
//! * each port name in the body is rewritten to the bound external node,
//! * `{param}` / bare-identifier parameter references are substituted from the
//!   instance parameters (and any `.param` defaults inside the body),
//! * internal element instance names are prefixed with the instance name to
//!   avoid collisions when a subcircuit is used more than once.

use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

use crate::error::SpiceError;
use crate::model::{Element, Subckt};
use crate::parser::parse_element;

/// Substitute a single token against the port→node map and the parameter map.
fn substitute_token(
    tok: &str,
    node_map: &BTreeMap<String, String>,
    params: &BTreeMap<String, String>,
) -> String {
    // `{param}` form.
    if let Some(inner) = tok.strip_prefix('{').and_then(|s| s.strip_suffix('}')) {
        if let Some(v) = params.get(inner) {
            return v.clone();
        }
        if let Some(v) = node_map.get(inner) {
            return v.clone();
        }
        return tok.to_string();
    }
    // Bare-identifier substitution (ports take precedence over params).
    if let Some(v) = node_map.get(tok) {
        return v.clone();
    }
    if let Some(v) = params.get(tok) {
        return v.clone();
    }
    tok.to_string()
}

/// Expand a single `X` instance into its flattened element cards.
///
/// Returns an error if the instance has no subcircuit name or references an
/// unknown subcircuit. Lines that fail to parse as elements (e.g. nested
/// `.subckt`/`.ends`/`.param`) are handled internally and not emitted.
pub fn expand_subckt_instance(
    inst: &Element,
    subckts: &[Subckt],
) -> Result<Vec<Element>, SpiceError> {
    let name = inst
        .subckt
        .as_ref()
        .ok_or(SpiceError::MissingField("subckt name"))?;
    let sub = subckts
        .iter()
        .find(|s| s.name.eq_ignore_ascii_case(name))
        .ok_or_else(|| SpiceError::Unsupported("unknown subcircuit in expansion"))?;

    // Parameter map: instance overrides body defaults.
    let mut params: BTreeMap<String, String> = BTreeMap::new();
    for p in &inst.params {
        if let Some((k, v)) = p.split_once('=') {
            params.insert(k.trim().to_string(), v.trim().to_string());
        }
    }

    // Port → external node mapping.
    let mut node_map: BTreeMap<String, String> = BTreeMap::new();
    for (i, port) in sub.ports.iter().enumerate() {
        let ext = inst.nodes.get(i).cloned().unwrap_or_default();
        node_map.insert(port.clone(), ext);
    }

    let mut out: Vec<Element> = Vec::new();
    for line in &sub.body {
        let toks: Vec<&str> = line.split_whitespace().collect();
        if toks.is_empty() {
            continue;
        }
        // `.param k=v` lines seed defaults and are not emitted.
        if toks[0].eq_ignore_ascii_case(".param") {
            for t in &toks[1..] {
                if let Some((k, v)) = t.split_once('=') {
                    params.entry(k.trim().to_string()).or_insert_with(|| v.trim().to_string());
                }
            }
            continue;
        }
        if toks[0].starts_with('.') {
            continue; // nested .subckt / .ends / other control lines
        }

        let mut new_tokens: Vec<String> = Vec::new();
        for (ti, tok) in toks.iter().enumerate() {
            if ti == 0 {
                // Prefix the instance name to keep it unique.
                new_tokens.push(format!("{}_{}", tok, inst.name));
            } else {
                new_tokens.push(substitute_token(tok, &node_map, &params));
            }
        }
        let line2 = new_tokens.join(" ");
        let toks2: Vec<&str> = line2.split_whitespace().collect();
        if let Ok(el) = parse_element(&toks2) {
            out.push(el);
        }
    }
    Ok(out)
}

/// Flatten a netlist one level: every `X` instance is replaced by the expanded
/// body of its referenced subcircuit (non-`X` elements are passed through).
///
/// Multi-level (nested) subcircuits are not recursively expanded; call this in
/// a loop if deeper flattening is required.
pub fn expand_netlist(
    elements: &[Element],
    subckts: &[Subckt],
) -> Result<Vec<Element>, SpiceError> {
    let mut out: Vec<Element> = Vec::new();
    for el in elements {
        if el.kind == crate::model::ElementKind::X {
            out.extend(expand_subckt_instance(el, subckts)?);
        } else {
            out.push(el.clone());
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{ElementKind, SpiceNetlist};
    use crate::parser::parse_str;

    const NET: &str = "\
X1 in out buf
X2 out load buf
.subckt buf (gain) a y
R1 a y {gain}
C1 y 0 10p
.ends
.subckt load p
R1 p 0 1k
.ends
";

    #[test]
    fn expands_ports_and_params() {
        let net = parse_str(NET).unwrap();
        let flat = expand_netlist(&net.elements, &net.subckts).unwrap();
        // Two X instances -> 2 elements each (R1, C1 for buf; R1 for load).
        assert_eq!(flat.len(), 4);
        // buf's R1 referenced param `gain` left verbatim (no instance value).
        let r1 = flat.iter().find(|e| e.name == "R1_X1").unwrap();
        assert_eq!(r1.nodes, vec!["in", "out"]);
        assert_eq!(r1.value.as_ref().unwrap().raw, "{gain}");
    }

    #[test]
    fn expands_with_instance_param() {
        let src = "\
X1 in out inv W=2k
.subckt inv (W) a y
R1 a y {W}
.ends
";
        let net = parse_str(src).unwrap();
        let flat = expand_netlist(&net.elements, &net.subckts).unwrap();
        let r1 = flat.iter().find(|e| e.name == "R1_X1").unwrap();
        assert_eq!(r1.value.as_ref().unwrap().raw, "2k");
        assert_eq!(r1.value.as_ref().unwrap().scalar, Some(2000.0));
    }

    #[test]
    fn unknown_subckt_errors() {
        let mut net = SpiceNetlist::default();
        net.elements.push(Element {
            kind: ElementKind::X,
            name: "X1".into(),
            nodes: vec!["a".into(), "b".into()],
            value: None,
            params: vec![],
            subckt: Some("nope".into()),
        });
        assert!(expand_netlist(&net.elements, &net.subckts).is_err());
    }
}
