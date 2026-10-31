//! Writer for MATPOWER case scripts. Output re-reads to an equal [`Case`].

use alloc::string::String;
use alloc::vec::Vec;
use core::fmt::Write as _;

use crate::model::{Branch, Bus, Case, Gen, Value};

/// Serialise a case as a MATPOWER script.
pub fn write(case: &Case) -> Vec<u8> {
    let mut out = String::new();
    if let Some(name) = &case.name {
        let _ = writeln!(out, "function mpc = {name}");
    }
    if let Some(v) = &case.version {
        let _ = writeln!(out, "mpc.version = '{}';", v.replace('\'', "''"));
    }
    let _ = writeln!(out, "mpc.baseMVA = {};", case.base_mva);
    out.push_str("mpc.bus = [\n");
    for b in &case.bus {
        bus_line(&mut out, b);
    }
    out.push_str("];\n");
    out.push_str("mpc.gen = [\n");
    for g in &case.gen {
        gen_line(&mut out, g);
    }
    out.push_str("];\n");
    out.push_str("mpc.branch = [\n");
    for br in &case.branch {
        branch_line(&mut out, br);
    }
    out.push_str("];\n");
    for (key, value) in &case.extra {
        write_value(&mut out, key, value);
    }
    out.into_bytes()
}

fn bus_line(out: &mut String, b: &Bus) {
    let _ = writeln!(
        out,
        "\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{};",
        b.id, b.kind, b.pd, b.qd, b.gs, b.bs, b.area, b.vm, b.va, b.base_kv, b.zone, b.vmax, b.vmin
    );
}

fn gen_line(out: &mut String, g: &Gen) {
    let _ = writeln!(
        out,
        "\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{};",
        g.bus, g.pg, g.qg, g.qmax, g.qmin, g.vg, g.mbase, g.status, g.pmax, g.pmin
    );
}

fn branch_line(out: &mut String, br: &Branch) {
    let _ = writeln!(
        out,
        "\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{};",
        br.from,
        br.to,
        br.r,
        br.x,
        br.b,
        br.rate_a,
        br.rate_b,
        br.rate_c,
        br.ratio,
        br.angle,
        br.status,
        br.angmin,
        br.angmax
    );
}

fn write_value(out: &mut String, key: &str, value: &Value) {
    match value {
        Value::Scalar(x) => {
            let _ = writeln!(out, "mpc.{key} = {x};");
        }
        Value::Text(t) => {
            let _ = writeln!(out, "mpc.{key} = '{}';", t.replace('\'', "''"));
        }
        Value::Matrix(rows) => {
            let _ = writeln!(out, "mpc.{key} = [");
            for row in rows {
                let cells: Vec<String> = row.iter().map(|x| alloc::format!("{x}")).collect();
                let _ = writeln!(out, "\t{};", cells.join("\t"));
            }
            out.push_str("];\n");
        }
        Value::Cell(items) => {
            let _ = writeln!(out, "mpc.{key} = {{");
            for item in items {
                let _ = writeln!(out, "'{}';", item.replace('\'', "''"));
            }
            out.push_str("};\n");
        }
    }
}
