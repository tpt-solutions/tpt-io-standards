//! Serialization of a [`SpiceNetlist`] back to SPICE (SPICE3-style) text.
//!
//! The output is canonical and round-trippable: re-parsing it yields a
//! structurally identical [`SpiceNetlist`].

use alloc::format;
use alloc::string::String;
use alloc::string::ToString;
use alloc::vec::Vec;

use crate::model::{Analysis, Element, Model, SpiceNetlist, Subckt};

/// Serialize a [`SpiceNetlist`] into SPICE source text.
pub fn serialize(net: &SpiceNetlist) -> Result<Vec<u8>, crate::error::SpiceError> {
    let mut out = String::new();

    if let Some(title) = &net.title {
        out.push('*');
        out.push(' ');
        out.push_str(title);
        out.push('\n');
    }
    if let Some(t) = net.temperature {
        out.push_str(&format!(".temp {t}\n"));
    }
    for el in &net.elements {
        out.push_str(&serialize_element(el));
        out.push('\n');
    }
    for m in &net.models {
        out.push_str(&serialize_model(m));
        out.push('\n');
    }
    for sub in &net.subckts {
        out.push_str(&serialize_subckt(sub));
        out.push('\n');
    }
    for a in &net.analyses {
        out.push_str(&serialize_analysis(a));
        out.push('\n');
    }
    out.push_str(".end\n");
    Ok(out.into_bytes())
}

fn serialize_element(el: &Element) -> String {
    let mut s = el.name.clone();
    for n in &el.nodes {
        s.push(' ');
        s.push_str(n);
    }
    if el.kind == crate::model::ElementKind::X {
        if let Some(sub) = &el.subckt {
            s.push(' ');
            s.push_str(sub);
        }
        for p in &el.params {
            s.push(' ');
            s.push_str(p);
        }
    } else {
        if let Some(v) = &el.value {
            s.push(' ');
            s.push_str(&v.raw);
        }
        for p in &el.params {
            s.push(' ');
            s.push_str(p);
        }
    }
    s
}

fn serialize_model(m: &Model) -> String {
    let params = m
        .params
        .iter()
        .map(|(k, v)| if v.is_empty() { k.clone() } else { format!("{k}={v}") })
        .collect::<Vec<_>>()
        .join(" ");
    format!(".model {} {} ({})", m.name, m.device, params)
}

fn serialize_subckt(sub: &Subckt) -> String {
    let mut s = format!(".subckt {}", sub.name);
    if !sub.params.is_empty() {
        s.push_str(&format!(" ({})", sub.params.join(",")));
    }
    for p in &sub.ports {
        s.push(' ');
        s.push_str(p);
    }
    let mut body = s;
    body.push('\n');
    for line in &sub.body {
        body.push_str(line);
        body.push('\n');
    }
    body.push_str(".ends");
    body
}

fn serialize_analysis(a: &Analysis) -> String {
    match a {
        Analysis::Tran(args) => format!(".tran {}", args.join(" ")),
        Analysis::Ac(args) => format!(".ac {}", args.join(" ")),
        Analysis::Op => ".op".to_string(),
        Analysis::Dc(args) => format!(".dc {}", args.join(" ")),
        Analysis::Noise(args) => format!(".noise {}", args.join(" ")),
        Analysis::Other(name, args) => format!(".{name} {}", args.join(" ")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse_str;

    const NETLIST: &str = "\
* RC low-pass filter
R1 in out 1k
C1 out 0 100n
V1 in 0 DC 5 AC 1
M1 d g s b nmos W=2u L=0.5u
.model nmos nmos (VTO=0.7 KP=120u)
.tran 1u 10m
.subckt inv a y vdd vss
R1 a y 1k
.ends
";

    #[test]
    fn round_trips() {
        let net = parse_str(NETLIST).unwrap();
        let bytes = serialize(&net).unwrap();
        let reparsed = parse_str(core::str::from_utf8(&bytes).unwrap()).unwrap();
        assert_eq!(net, reparsed);
    }
}
