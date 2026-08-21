//! Feature-gated integration stub for the future `tpt-eng-geotech` crate.
//!
//! `tpt-eng-geotech` is not yet published (per `todo.md`). This module provides
//! only the conversion boundary — a stable API plus a local placeholder type — so
//! the `eng-geotech-integration` feature flag compiles and links today.

#[cfg(feature = "eng-geotech-integration")]
pub mod geotech {
    //! Stub conversion into a future `tpt_eng_geotech` geotechnical type.
    use crate::model::WellLog;

    /// Placeholder for a future `tpt_eng_geotech::BoreholeLog`.
    #[derive(Debug, Clone, PartialEq, Default)]
    pub struct BoreholeStub {
        /// Well name (if known).
        pub well: Option<alloc::string::String>,
        /// Number of curves (channels).
        pub curves: usize,
        /// Number of depth-indexed samples.
        pub samples: usize,
    }

    /// Convert a parsed LAS well-log into a borehole placeholder.
    pub fn to_borehole(log: &WellLog) -> BoreholeStub {
        BoreholeStub {
            well: log.well.well_name.clone(),
            curves: log.curves.len(),
            samples: log.data.len(),
        }
    }
}
