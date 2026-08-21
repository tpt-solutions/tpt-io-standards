//! Feature-gated integration stubs for downstream engineering crates.
//!
//! `tpt-eng-pcb` and `tpt-eng-geometry` are not yet published (per `todo.md`).
//! These modules therefore provide only the *conversion boundary* — a stable
//! API surface plus a local placeholder type — so the `eng-pcb-integration` and
//! `eng-geometry-integration` feature flags compile and link today. Once the
//! real crates exist, only the function bodies need to change.

#[cfg(feature = "eng-pcb-integration")]
pub mod eng_pcb {
    //! Stub conversion into a future `tpt-eng-pcb` PCB layer type.
    use crate::model::{GerberFile, Polarity};

    /// Placeholder for a future `tpt_eng_pcb::PcbLayer`.
    #[derive(Debug, Clone, PartialEq, Default)]
    pub struct PcbLayerStub {
        /// Number of defined apertures.
        pub apertures: usize,
        /// Number of graphic objects.
        pub objects: usize,
        /// Layer polarity as a string.
        pub polarity: &'static str,
    }

    /// Convert a parsed Gerber file into a PCB-layer placeholder.
    pub fn to_pcb_layer(file: &GerberFile) -> PcbLayerStub {
        let polarity = match file.polarity {
            Polarity::Positive => "positive",
            Polarity::Negative => "negative",
        };
        PcbLayerStub { apertures: file.apertures.len(), objects: file.objects.len(), polarity }
    }
}

#[cfg(feature = "eng-geometry-integration")]
pub mod eng_geometry {
    //! Stub conversion into a future `tpt_eng_geometry` geometry type.
    use crate::model::{GerberFile, ObjectKind};

    /// Placeholder for a future `tpt_eng_geometry::Geometry`.
    #[derive(Debug, Clone, PartialEq, Default)]
    pub struct GeometryStub {
        /// Total vertex count across all objects.
        pub vertex_count: usize,
    }

    /// Flatten a parsed Gerber file's graphic objects into a geometry placeholder.
    pub fn to_geometry(file: &GerberFile) -> GeometryStub {
        let mut vertex_count = 0usize;
        for obj in &file.objects {
            vertex_count += match &obj.kind {
                ObjectKind::Region { points } => points.len().max(1),
                _ => 1,
            };
        }
        GeometryStub { vertex_count }
    }
}
