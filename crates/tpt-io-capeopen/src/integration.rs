//! Feature-gated integration stubs for the future `tpt-thermo` and
//! `tpt-eng-unit-ops` crates.
//!
//! Neither downstream crate is published yet (per `todo.md`). These modules
//! provide only the conversion boundary — a stable API plus a local placeholder
//! type — so the `thermo-integration` / `eng-unit-ops-integration` feature flags
//! compile and link today. When the real crates exist, only the function bodies
//! need to change.

#[cfg(feature = "thermo-integration")]
pub mod thermo {
    //! Stub conversion from a CAPE-OPEN [`PropertyPackage`] into a future
    //! `tpt_thermo` model.
    use crate::model::PropertyPackage;

    /// Placeholder for a future `tpt_thermo::PropertyModel`.
    #[derive(Debug, Clone, PartialEq, Default)]
    pub struct PropertyModelStub {
        /// Package name.
        pub name: String,
        /// Number of components in the package.
        pub n_components: usize,
        /// Number of base properties the package supports (single + two phase).
        pub n_properties: usize,
    }

    /// Convert a CAPE-OPEN property package into a thermodynamic model placeholder.
    pub fn to_property_model(pp: &PropertyPackage) -> Option<PropertyModelStub> {
        pp.validate().ok()?;
        Some(PropertyModelStub {
            name: pp.name.clone(),
            n_components: pp.components.len(),
            n_properties: pp.supported_single_phase.len() + pp.supported_two_phase.len(),
        })
    }
}

#[cfg(feature = "eng-unit-ops-integration")]
pub mod unit_ops {
    //! Stub conversion from a CAPE-OPEN [`Flowsheet`] into a future
    //! `tpt_eng_unit_ops` model.
    use crate::flowsheet::Flowsheet;

    /// Placeholder for a future `tpt_eng_unit_ops::ProcessModel`.
    #[derive(Debug, Clone, PartialEq, Default)]
    pub struct ProcessModelStub {
        /// Number of unit operations.
        pub units: usize,
        /// Number of streams.
        pub streams: usize,
        /// Whether the flowsheet topology validated.
        pub connected: bool,
    }

    /// Convert a CAPE-OPEN flowsheet into a unit-operations model placeholder.
    pub fn to_process_model(fs: &Flowsheet) -> ProcessModelStub {
        let connected = fs.validate().is_ok();
        ProcessModelStub { units: fs.unit_operations.len(), streams: fs.streams.len(), connected }
    }
}
