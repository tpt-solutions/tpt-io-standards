//! Typed view of a MATPOWER case: the `bus`, `gen` and `branch` matrices plus any
//! other fields, kept in their original order.

use alloc::string::{String, ToString};
use alloc::vec::Vec;

use crate::error::MatpowerError;

/// A value assigned to a field of `mpc`.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    /// A single number, e.g. `mpc.baseMVA = 100;`.
    Scalar(f64),
    /// A quoted string, e.g. `mpc.version = '2';`.
    Text(String),
    /// A numeric matrix; rows may differ in length (as in `gencost`).
    Matrix(Vec<Vec<f64>>),
    /// A cell array of strings, e.g. `mpc.bus_name = {'North'; 'South'};`.
    Cell(Vec<String>),
}

/// One row of the `bus` matrix (MATPOWER column order, columns 1–13).
#[derive(Debug, Clone, PartialEq)]
pub struct Bus {
    /// Bus number (column 1).
    pub id: u32,
    /// Bus type: 1 = PQ, 2 = PV, 3 = reference, 4 = isolated (column 2).
    pub kind: u32,
    /// Real power demand, MW (column 3).
    pub pd: f64,
    /// Reactive power demand, MVAr (column 4).
    pub qd: f64,
    /// Shunt conductance, MW at 1.0 pu voltage (column 5).
    pub gs: f64,
    /// Shunt susceptance, MVAr at 1.0 pu voltage (column 6).
    pub bs: f64,
    /// Area number (column 7).
    pub area: u32,
    /// Voltage magnitude, pu (column 8).
    pub vm: f64,
    /// Voltage angle, degrees (column 9).
    pub va: f64,
    /// Base voltage, kV (column 10).
    pub base_kv: f64,
    /// Loss zone (column 11).
    pub zone: u32,
    /// Maximum voltage magnitude, pu (column 12).
    pub vmax: f64,
    /// Minimum voltage magnitude, pu (column 13).
    pub vmin: f64,
}

/// One row of the `gen` matrix (columns 1–10).
#[derive(Debug, Clone, PartialEq)]
pub struct Gen {
    /// Bus the generator is connected to (column 1).
    pub bus: u32,
    /// Real power output, MW (column 2).
    pub pg: f64,
    /// Reactive power output, MVAr (column 3).
    pub qg: f64,
    /// Maximum reactive output, MVAr (column 4).
    pub qmax: f64,
    /// Minimum reactive output, MVAr (column 5).
    pub qmin: f64,
    /// Voltage setpoint, pu (column 6).
    pub vg: f64,
    /// Machine base, MVA (column 7).
    pub mbase: f64,
    /// 1 = in service, 0 = out of service (column 8).
    pub status: u32,
    /// Maximum real output, MW (column 9).
    pub pmax: f64,
    /// Minimum real output, MW (column 10).
    pub pmin: f64,
}

/// One row of the `branch` matrix (columns 1–13).
#[derive(Debug, Clone, PartialEq)]
pub struct Branch {
    /// From bus (column 1).
    pub from: u32,
    /// To bus (column 2).
    pub to: u32,
    /// Series resistance, pu (column 3).
    pub r: f64,
    /// Series reactance, pu (column 4).
    pub x: f64,
    /// Total line charging susceptance, pu (column 5).
    pub b: f64,
    /// Long-term rating, MVA (column 6).
    pub rate_a: f64,
    /// Short-term rating, MVA (column 7).
    pub rate_b: f64,
    /// Emergency rating, MVA (column 8).
    pub rate_c: f64,
    /// Transformer off-nominal turns ratio, 0 for a line (column 9).
    pub ratio: f64,
    /// Transformer phase shift, degrees (column 10).
    pub angle: f64,
    /// 1 = in service, 0 = out of service (column 11).
    pub status: u32,
    /// Minimum angle difference, degrees (column 12; defaults to -360).
    pub angmin: f64,
    /// Maximum angle difference, degrees (column 13; defaults to 360).
    pub angmax: f64,
}

/// A MATPOWER power-flow case.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Case {
    /// Function name from the `function mpc = name` header, if present.
    pub name: Option<String>,
    /// The `mpc.version` string, e.g. `"2"`.
    pub version: Option<String>,
    /// The `mpc.baseMVA` system base, MVA.
    pub base_mva: f64,
    /// Bus rows.
    pub bus: Vec<Bus>,
    /// Generator rows.
    pub gen: Vec<Gen>,
    /// Branch rows.
    pub branch: Vec<Branch>,
    /// Every other `mpc.*` field (for example `gencost`, `bus_name`), in file order.
    pub extra: Vec<(String, Value)>,
}

impl Case {
    /// Check the rules a power-flow solver relies on.
    ///
    /// Requires a positive base, unique bus numbers, exactly one reference bus,
    /// valid bus types and status flags, and that every generator and branch end
    /// refers to a bus in the case.
    pub fn validate(&self) -> Result<(), MatpowerError> {
        if !(self.base_mva.is_finite() && self.base_mva > 0.0) {
            return Err(invalid("baseMVA must be a positive, finite number"));
        }
        let mut ids: Vec<u32> = self.bus.iter().map(|b| b.id).collect();
        ids.sort_unstable();
        if ids.windows(2).any(|w| w[0] == w[1]) {
            return Err(invalid("duplicate bus number"));
        }
        let mut references = 0usize;
        for b in &self.bus {
            if !(1..=4).contains(&b.kind) {
                return Err(invalid(
                    "bus type must be 1 (PQ), 2 (PV), 3 (reference) or 4 (isolated)",
                ));
            }
            if b.kind == 3 {
                references += 1;
            }
        }
        if !self.bus.is_empty() && references != 1 {
            return Err(invalid("a case needs exactly one reference bus (type 3)"));
        }
        for g in &self.gen {
            if ids.binary_search(&g.bus).is_err() {
                return Err(invalid("generator refers to a bus that is not in the case"));
            }
            if g.status > 1 {
                return Err(invalid("generator status must be 0 or 1"));
            }
        }
        for br in &self.branch {
            if ids.binary_search(&br.from).is_err() || ids.binary_search(&br.to).is_err() {
                return Err(invalid("branch refers to a bus that is not in the case"));
            }
            if br.status > 1 {
                return Err(invalid("branch status must be 0 or 1"));
            }
        }
        Ok(())
    }

    /// The value of an extra field by name, e.g. `"gencost"`.
    pub fn extra_field(&self, name: &str) -> Option<&Value> {
        self.extra.iter().find(|(k, _)| k == name).map(|(_, v)| v)
    }
}

fn invalid(msg: &str) -> MatpowerError {
    MatpowerError::Invalid(msg.to_string())
}
