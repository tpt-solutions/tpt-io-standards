//! Feature-gated integration stub for the future `tpt-sci-grid` crate.
//!
//! `tpt-sci-grid` is not yet published (per `todo.md`). This module provides
//! only the conversion boundary — a stable API plus a local placeholder type — so
//! the `sci-grid-integration` feature flag compiles and links today.

#[cfg(feature = "sci-grid-integration")]
pub mod sci_grid {
    //! Stub conversion into a future `tpt_sci_grid` grid type.
    use crate::model::NetCdf;

    /// Placeholder for a future `tpt_sci_grid::Grid`.
    #[derive(Debug, Clone, PartialEq, Default)]
    pub struct GridStub {
        /// Number of dimensions.
        pub dims: usize,
        /// Number of variables.
        pub vars: usize,
    }

    /// Convert a parsed NetCDF document into a grid placeholder.
    pub fn to_grid(nc: &NetCdf) -> GridStub {
        GridStub { dims: nc.dims.len(), vars: nc.vars.len() }
    }
}
