//! Data model for parsed SPICE netlists.

use alloc::string::String;
use alloc::vec::Vec;

/// A parsed component value: the original token plus its scaled numeric form
/// (when it could be interpreted as a number with an SI suffix).
#[derive(Debug, Clone, PartialEq)]
pub struct Value {
    /// Original token text (e.g. `1k`, `2.2u`, `R=1k`).
    pub raw: String,
    /// Scaled numeric value, or `None` if the token was not a plain number
    /// (e.g. an expression `{Rval}` or a model reference).
    pub scalar: Option<f64>,
}

/// Element (instance) designator family.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ElementKind {
    /// Resistor (`R`).
    R,
    /// Inductor (`L`).
    L,
    /// Capacitor (`C`).
    C,
    /// Independent voltage source (`V`).
    V,
    /// Independent current source (`I`).
    I,
    /// Diode (`D`).
    D,
    /// Bipolar transistor (`Q`).
    Q,
    /// Mosfet (`M`).
    M,
    /// Voltage-controlled voltage source (`E`).
    E,
    /// Current-controlled current source (`F`).
    F,
    /// Voltage-controlled current source (`G`).
    G,
    /// Current-controlled voltage source (`H`).
    H,
    /// Transmission line (`T`).
    T,
    /// Subcircuit instance (`X`).
    X,
    /// Behavioral source (`B`).
    B,
    /// Lossy transmission / general (`O`).
    O,
}

impl ElementKind {
    /// Classify the first character of an instance name.
    pub fn from_designator(c: char) -> Option<ElementKind> {
        Some(match c.to_ascii_uppercase() {
            'R' => ElementKind::R,
            'L' => ElementKind::L,
            'C' => ElementKind::C,
            'V' => ElementKind::V,
            'I' => ElementKind::I,
            'D' => ElementKind::D,
            'Q' => ElementKind::Q,
            'M' => ElementKind::M,
            'E' => ElementKind::E,
            'F' => ElementKind::F,
            'G' => ElementKind::G,
            'H' => ElementKind::H,
            'T' => ElementKind::T,
            'X' => ElementKind::X,
            'B' => ElementKind::B,
            'O' => ElementKind::O,
            _ => return None,
        })
    }
}

/// A single circuit element / instance card.
#[derive(Debug, Clone, PartialEq)]
pub struct Element {
    /// Element family.
    pub kind: ElementKind,
    /// Full instance name (designator + index), e.g. `R1`, `M2`.
    pub name: String,
    /// Connected nodes, in card order.
    pub nodes: Vec<String>,
    /// Parsed value for `R`/`L`/`C` elements (first non-node token), if present.
    pub value: Option<Value>,
    /// Remaining tokens (model name, parameters, gain, …) in card order.
    pub params: Vec<String>,
    /// For `X` (subcircuit) instances, the referenced subcircuit name.
    pub subckt: Option<String>,
}

impl Element {
    /// Look up a `key=value` parameter by key, returning its parsed `f64` value.
    pub fn param_f64(&self, key: &str) -> Option<f64> {
        self.params.iter().find_map(|p| {
            let (k, v) = p.split_once('=')?;
            if k.eq_ignore_ascii_case(key) {
                v.trim().parse::<f64>().ok()
            } else {
                None
            }
        })
    }

    /// Resolve the effective value of a passive `R`/`L`/`C` element at
    /// `temp_celsius`, applying a linear/quadratic temperature coefficient
    /// (`TC`/`TC1` and optional `TC2`) around the nominal temperature (`TEMP`,
    /// default 27 °C). Returns `None` if the element has no numeric value.
    pub fn value_at_temperature(&self, temp_celsius: f64) -> Option<f64> {
        let r0 = self.value.as_ref()?.scalar?;
        let tc1 = self.param_f64("TC").or_else(|| self.param_f64("TC1"));
        let tc2 = self.param_f64("TC2");
        if tc1.is_none() && tc2.is_none() {
            return Some(r0);
        }
        let t0 = self.param_f64("TEMP").unwrap_or(27.0);
        let dt = temp_celsius - t0;
        let factor = 1.0 + tc1.unwrap_or(0.0) * dt + tc2.unwrap_or(0.0) * dt * dt;
        Some(r0 * factor)
    }
}

/// A `.model` device model definition.
#[derive(Debug, Clone, PartialEq)]
pub struct Model {
    /// Model name (referenced by elements).
    pub name: String,
    /// Device type (e.g. `npn`, `pmos`, `d`).
    pub device: String,
    /// Parameter assignments (`key = value`).
    pub params: Vec<(String, String)>,
}

/// A `.subckt` / `.ends` definition.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Subckt {
    /// Subcircuit name.
    pub name: String,
    /// Formal parameters declared in the header, e.g. `(w,l)` in
    /// `.subckt inv (w,l) a y vdd vss`.
    pub params: Vec<String>,
    /// Port nodes.
    pub ports: Vec<String>,
    /// Raw body lines (between `.subckt` and `.ends`), preserved verbatim.
    pub body: Vec<String>,
}

/// An analysis (simulation) command.
#[derive(Debug, Clone, PartialEq)]
pub enum Analysis {
    /// `.tran` transient analysis.
    Tran(Vec<String>),
    /// `.ac` AC analysis.
    Ac(Vec<String>),
    /// `.op` operating point.
    Op,
    /// `.dc` DC sweep.
    Dc(Vec<String>),
    /// `.noise` noise analysis.
    Noise(Vec<String>),
    /// Any other `.` command, preserved by name + args.
    Other(String, Vec<String>),
}

/// A fully parsed SPICE netlist.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SpiceNetlist {
    /// Title line (first non-comment line).
    pub title: Option<String>,
    /// Global simulation temperature (°C), set by a `.temp` statement.
    pub temperature: Option<f64>,
    /// All element / instance cards.
    pub elements: Vec<Element>,
    /// Model definitions.
    pub models: Vec<Model>,
    /// Subcircuit definitions.
    pub subckts: Vec<Subckt>,
    /// Analysis commands.
    pub analyses: Vec<Analysis>,
}
