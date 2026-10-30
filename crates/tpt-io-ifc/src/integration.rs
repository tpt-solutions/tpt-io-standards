//! Feature-gated integration stubs for downstream engineering crates.
//!
//! `tpt-eng-geometry` and `tpt-eng-cad` are not yet published (per `todo.md`).
//! These modules therefore provide only the *conversion boundary* — a stable
//! API surface plus a local placeholder type — so the `eng-geometry-integration`
//! and `eng-cad-integration` feature flags compile and link today. Once the
//! real crates exist, only the function bodies need to change.

#[cfg(feature = "eng-geometry-integration")]
pub mod eng_geometry {
    //! Stub conversion into a future `tpt-eng-geometry` geometry type.
    use crate::geometry::{Geometry, GeometryExtractor, Mesh};
    use crate::model::IfcFile;

    /// Placeholder for a future `tpt_eng_geometry::Geometry`.
    #[derive(Debug, Clone, PartialEq, Default)]
    pub struct GeometryStub {
        /// Total vertex count across all extracted meshes.
        pub vertex_count: usize,
        /// Total face count across all extracted meshes.
        pub face_count: usize,
        /// Number of CSG nodes encountered.
        pub csg_node_count: usize,
    }

    /// Extract all geometry from a file and reduce it to a geometry placeholder.
    pub fn to_geometry(file: &IfcFile) -> GeometryStub {
        let extractor = GeometryExtractor::new(file);
        let mut stub = GeometryStub::default();
        for inst in &file.instances {
            if let Ok(geom) = extractor.extract_item(inst) {
                match geom {
                    Geometry::BRep(mesh) | Geometry::Tessellated(mesh) => {
                        count_mesh(&mesh, &mut stub);
                    }
                    Geometry::Csg(node) => {
                        stub.csg_node_count += count_csg(&node);
                    }
                }
            }
        }
        stub
    }

    fn count_mesh(mesh: &Mesh, stub: &mut GeometryStub) {
        stub.vertex_count += mesh.vertex_count();
        stub.face_count += mesh.face_count();
    }

    fn count_csg(node: &crate::geometry::CsgNode) -> usize {
        use crate::geometry::CsgNode;
        match node {
            CsgNode::Primitive { .. } => 1,
            CsgNode::Union { left, right }
            | CsgNode::Intersection { left, right }
            | CsgNode::Difference { left, right } => 1 + count_csg(left) + count_csg(right),
        }
    }
}

#[cfg(feature = "eng-cad-integration")]
pub mod eng_cad {
    //! Stub conversion into a future `tpt-eng-cad` CAD model type.
    use crate::model::IfcFile;
    use crate::spatial::SpatialNavigator;

    /// Placeholder for a future `tpt_eng_cad::CadModel`.
    #[derive(Debug, Clone, PartialEq, Default)]
    pub struct CadModelStub {
        /// Number of spatial structure nodes (site/building/storey/space).
        pub spatial_nodes: usize,
        /// Number of building elements (walls, slabs, …).
        pub element_count: usize,
    }

    /// Convert a parsed IFC file into a CAD-model placeholder.
    pub fn to_cad_model(file: &IfcFile) -> CadModelStub {
        let nav = SpatialNavigator::new(file);
        let mut spatial_nodes = 0usize;
        if let Ok(trees) = nav.project_tree() {
            for tree in &trees {
                spatial_nodes += count_spatial(tree);
            }
        }
        CadModelStub {
            spatial_nodes,
            element_count: file
                .instances
                .iter()
                .filter(|i| i.type_name.starts_with("IFC") && i.type_name != "IFCPROJECT")
                .count(),
        }
    }

    fn count_spatial(node: &crate::spatial::SpatialNode) -> usize {
        1 + node.children.iter().map(count_spatial).sum::<usize>()
    }
}
