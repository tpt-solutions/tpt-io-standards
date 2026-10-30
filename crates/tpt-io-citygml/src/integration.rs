//! Feature-gated integration stubs for downstream engineering crates.
//!
//! `tpt-eng-geometry` is not yet published (per `todo.md`). This module
//! provides only the *conversion boundary* — a stable API surface plus a
//! local placeholder type — so the `eng-geometry-integration` feature flag
//! compiles and links today. Once the real crate exists, only the function
//! bodies need to change.

#[cfg(feature = "eng-geometry-integration")]
pub mod eng_geometry {
    //! Stub conversion into a future `tpt_eng_geometry` geometry type.
    use crate::model::{CityModel, Geometry, Point3};

    /// Placeholder for a future `tpt_eng_geometry::Geometry`.
    #[derive(Debug, Clone, PartialEq, Default)]
    pub struct GeometryStub {
        /// Total vertex count across all city objects.
        pub vertex_count: usize,
        /// Total surface count.
        pub surface_count: usize,
    }

    /// Flatten a parsed CityModel into a geometry placeholder.
    pub fn to_geometry(model: &CityModel) -> GeometryStub {
        let mut vertex_count = 0usize;
        let mut surface_count = 0usize;
        for obj in &model.objects {
            for geom in &obj.geometries {
                surface_count += geom.surfaces.len();
                for surface in &geom.surfaces {
                    if let Some(ring) = &surface.exterior {
                        vertex_count += ring.points.len();
                    }
                    for ring in &surface.interiors {
                        vertex_count += ring.points.len();
                    }
                }
            }
        }
        GeometryStub { vertex_count, surface_count }
    }

    /// Extract all unique vertices from a geometry.
    pub fn extract_vertices(geom: &Geometry) -> Vec<Point3> {
        let mut pts = Vec::new();
        for surface in &geom.surfaces {
            if let Some(ring) = &surface.exterior {
                pts.extend_from_slice(&ring.points);
            }
            for ring in &surface.interiors {
                pts.extend_from_slice(&ring.points);
            }
        }
        pts
    }
}
