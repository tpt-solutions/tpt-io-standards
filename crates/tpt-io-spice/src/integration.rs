//! Feature-gated integration stub for the future `tpt-em-circuit` crate.
//!
//! `tpt-em-circuit` is not yet published (per `todo.md`). This module provides
//! only the *conversion boundary* — a stable API surface plus a local
//! placeholder type — so the `em-circuit-integration` feature flag compiles and
//! links today. Once the real crate exists, only the function body needs to
//! change.

#[cfg(feature = "em-circuit-integration")]
pub mod em_circuit {
    //! Stub conversion into a future `tpt_em_circuit` netlist type.
    use crate::model::SpiceNetlist;

    /// Placeholder for a future `tpt_em_circuit::Circuit`.
    #[derive(Debug, Clone, PartialEq, Default)]
    pub struct CircuitStub {
        /// Number of elements (instances).
        pub elements: usize,
        /// Number of subcircuit definitions.
        pub subckts: usize,
        /// Number of model definitions.
        pub models: usize,
    }

    /// Convert a parsed SPICE netlist into a circuit placeholder.
    pub fn to_circuit(net: &SpiceNetlist) -> CircuitStub {
        CircuitStub {
            elements: net.elements.len(),
            subckts: net.subckts.len(),
            models: net.models.len(),
        }
    }
}
