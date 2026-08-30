//! Feature-gated integration stub for the future `tpt-em-circuit` crate.
//!
//! `tpt-em-circuit` is not yet published (per `todo.md`). This module provides
//! only the conversion boundary — a stable API plus a local placeholder type — so
//! the `em-circuit-integration` feature flag compiles and links today. When the
//! real crate exists, only the function bodies need to change.

#[cfg(feature = "em-circuit-integration")]
pub mod em_circuit {
    //! Stub conversion into a future `tpt_em_circuit` netlist type.
    use crate::flatten::FlatNetlist;
    use crate::model::Edif;

    /// Placeholder for a future `tpt_em_circuit::Circuit`.
    #[derive(Debug, Clone, PartialEq, Default)]
    pub struct CircuitStub {
        /// Number of primitive (leaf) instances.
        pub instances: usize,
        /// Number of flattened nets.
        pub nets: usize,
        /// Top cell name.
        pub top_cell: String,
    }

    /// Convert a parsed EDIF document's flattened netlist into a circuit placeholder.
    pub fn to_circuit(edif: &Edif) -> Option<CircuitStub> {
        let flat = crate::flatten::flatten(edif).ok()?;
        Some(CircuitStub {
            instances: count_leaves(&flat),
            nets: flat.nets.len(),
            top_cell: flat.top_cell,
        })
    }

    fn count_leaves(flat: &FlatNetlist) -> usize {
        flat.leaves.iter().filter(|l| !l.path.is_empty()).count()
    }
}
