//! Feature-gated integration stubs for downstream engineering crates.
//!
//! `tpt-eng-geometry` and `tpt-math-geometry` are not yet published (per
//! `todo.md`). These modules provide only the *conversion boundary* — a stable
//! API surface plus a local placeholder type — so the `eng-geometry-integration`
//! and `math-geometry-integration` feature flags compile and link today. Once
//! the real crates exist, only the function bodies need to change.

#[cfg(feature = "eng-geometry-integration")]
pub mod eng_geometry {
    //! Stub conversion into a future `tpt_eng_geometry` geometry type.

    use crate::geojson::{FeatureCollection, GeometryEnum};
    use crate::shapefile::ShapefileDataset;

    /// Placeholder for a future `tpt_eng_geometry::Geometry`.
    #[derive(Debug, Clone, PartialEq, Default)]
    pub struct GeometryStub {
        /// Total vertex count across all features/records.
        pub vertex_count: usize,
        /// Total ring/line count (polygonal parts).
        pub part_count: usize,
    }

    /// Flatten a GeoJSON [`FeatureCollection`] into a geometry placeholder.
    pub fn to_geometry(fc: &FeatureCollection) -> GeometryStub {
        let mut vertex_count = 0usize;
        let mut part_count = 0usize;
        for f in &fc.features {
            if let Some(g) = &f.geometry {
                count_geometry(&g.geometry, &mut vertex_count, &mut part_count);
            }
        }
        GeometryStub { vertex_count, part_count }
    }

    fn count_geometry(g: &GeometryEnum, verts: &mut usize, parts: &mut usize) {
        match g {
            GeometryEnum::Point(p) => *verts += p.0.len(),
            GeometryEnum::MultiPoint(ps) | GeometryEnum::LineString(ps) => {
                *parts += 1;
                for p in ps {
                    *verts += p.0.len();
                }
            }
            GeometryEnum::MultiLineString(ls) => {
                for line in ls {
                    *parts += 1;
                    *verts += line.iter().map(|p| p.0.len()).sum::<usize>();
                }
            }
            GeometryEnum::Polygon(rings) => {
                *parts += rings.len();
                for ring in rings {
                    *verts += ring.iter().map(|p| p.0.len()).sum::<usize>();
                }
            }
            GeometryEnum::MultiPolygon(polys) => {
                for poly in polys {
                    *parts += poly.len();
                    for ring in poly {
                        *verts += ring.iter().map(|p| p.0.len()).sum::<usize>();
                    }
                }
            }
            GeometryEnum::GeometryCollection(geoms) => {
                for g in geoms {
                    count_geometry(&g.geometry, verts, parts);
                }
            }
        }
    }

    /// Total vertex count of a parsed Shapefile dataset.
    pub fn shapefile_vertex_count(ds: &ShapefileDataset) -> usize {
        use crate::shapefile::geometry_points;
        ds.shapefile.records.iter().map(|r| geometry_points(&r.geometry).len()).sum()
    }
}

#[cfg(feature = "math-geometry-integration")]
pub mod math_geometry {
    //! Stub conversion into a future `tpt_math_geometry` coordinate type.

    use crate::crs::Crs;
    use crate::geojson::Position;

    /// Placeholder for a future `tpt_math_geometry::Point3`.
    #[derive(Debug, Clone, Copy, PartialEq, Default)]
    pub struct Point3Stub {
        /// X / longitude.
        pub x: f64,
        /// Y / latitude.
        pub y: f64,
        /// Z / elevation.
        pub z: f64,
    }

    impl From<&Position> for Point3Stub {
        fn from(p: &Position) -> Self {
            Point3Stub { x: p.longitude(), y: p.latitude(), z: p.elevation().unwrap_or(0.0) }
        }
    }

    /// Convert a GeoJSON position into a math-geometry stub point.
    pub fn to_point(p: &Position) -> Point3Stub {
        Point3Stub::from(p)
    }

    /// Describe a CRS as a math-geometry-compatible stub (code + kind).
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub struct CrsStub {
        /// EPSG code, if known.
        pub code: Option<u32>,
        /// Whether the CRS is projected.
        pub is_projected: bool,
    }

    /// Convert a [`Crs`] into a math-geometry stub.
    pub fn to_crs_stub(crs: &Crs) -> CrsStub {
        CrsStub { code: crs.code, is_projected: crs.kind == crate::crs::CrsKind::Projected }
    }
}
