//! Geometry extraction from IFC entity instances.
//!
//! IFC represents solid geometry in several flavours. This module walks the
//! representation graph of a product and produces a neutral, strongly-typed
//! output per representation item:
//!
//! - **B-Rep** (`IfcFacetedBrep`, `IfcClosedShell`, …): boundary
//!   representation as a set of polygonal faces.
//! - **CSG** (`IfcCsgSolid`, `IfcBooleanResult`): constructive solid
//!   geometry as a binary tree of primitives combined by union /
//!   intersection / difference.
//! - **Tessellated** (`IfcTriangulatedFaceSet`, `IfcPolygonalFaceSet`):
//!   pre-tessellated triangle/quad meshes.
//!
//! The extraction walks only the well-known structural entities; unsupported
//! item types are reported via [`IfcError::Unsupported`] so callers can decide
//! whether to skip or fail.

use alloc::boxed::Box;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

use crate::error::IfcError;
use crate::model::{EntityInstance, IfcFile, IfcValue};

/// A 3D point.
pub type Point3 = (f64, f64, f64);

/// A tessellated mesh: a flat list of vertices and a list of faces, each face
/// being a list of vertex indices.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Mesh {
    /// The vertex buffer.
    pub vertices: Vec<Point3>,
    /// The face buffer — each face is a list of vertex indices.
    pub faces: Vec<Vec<usize>>,
}

impl Mesh {
    /// Create an empty mesh.
    pub fn new() -> Self {
        Self::default()
    }

    /// Return the number of vertices.
    pub fn vertex_count(&self) -> usize {
        self.vertices.len()
    }

    /// Return the number of faces.
    pub fn face_count(&self) -> usize {
        self.faces.len()
    }

    /// Return `true` if the mesh has no vertices.
    pub fn is_empty(&self) -> bool {
        self.vertices.is_empty()
    }
}

/// A CSG tree node.
#[derive(Debug, Clone, PartialEq)]
pub enum CsgNode {
    /// A primitive (e.g. a block, cylinder) — stored as the entity type name.
    Primitive {
        /// The entity type name, e.g. `IfcBlock`.
        kind: String,
        /// The instance id of the CSG primitive.
        id: usize,
    },
    /// A union of two sub-trees.
    Union {
        /// Left operand.
        left: Box<CsgNode>,
        /// Right operand.
        right: Box<CsgNode>,
    },
    /// An intersection of two sub-trees.
    Intersection {
        /// Left operand.
        left: Box<CsgNode>,
        /// Right operand.
        right: Box<CsgNode>,
    },
    /// A difference: `left - right`.
    Difference {
        /// Left operand (the base).
        left: Box<CsgNode>,
        /// Right operand (the tool).
        right: Box<CsgNode>,
    },
}

/// A geometry representation extracted from a product.
#[derive(Debug, Clone, PartialEq)]
pub enum Geometry {
    /// A B-Rep representation, reduced to a mesh of polygonal faces.
    BRep(Mesh),
    /// A CSG representation.
    Csg(CsgNode),
    /// A pre-tessellated mesh.
    Tessellated(Mesh),
}

/// A geometry extractor that walks the representation graph of IFC products.
///
/// Build with [`GeometryExtractor::new`], then call [`GeometryExtractor::extract_product`] to
/// extract all geometry for a given product instance.
#[derive(Debug, Clone)]
pub struct GeometryExtractor<'a> {
    file: &'a IfcFile,
}

impl<'a> GeometryExtractor<'a> {
    /// Create a new extractor over `file`.
    pub fn new(file: &'a IfcFile) -> Self {
        Self { file }
    }

    /// Extract all geometry representations of a product.
    ///
    /// The product is expected to be an `IfcProduct` (or subclass) whose
    /// `Representation` attribute (arg 4) points to an
    /// `IfcProductRepresentation`. Returns a list of [`Geometry`] per
    /// representation item.
    pub fn extract_product(&self, product_id: usize) -> Result<Vec<Geometry>, IfcError> {
        let product = self
            .file
            .instance(product_id)
            .ok_or_else(|| IfcError::unknown_instance(0, product_id))?;
        let repr_val = product
            .arg(4)
            .ok_or_else(|| IfcError::relationship("IfcProduct: Representation (arg 4) required"))?;
        let repr_id = repr_val.as_ref().ok_or_else(|| {
            IfcError::relationship("IfcProduct: Representation (arg 4) must be a reference")
        })?;
        let repr =
            self.file.instance(repr_id).ok_or_else(|| IfcError::unknown_instance(0, repr_id))?;
        // IfcProductRepresentation = (Name, Description, Representations)
        let reps = repr.arg(2).and_then(IfcValue::as_list).ok_or_else(|| {
            IfcError::relationship(
                "IfcProductRepresentation: Representations (arg 2) must be a list",
            )
        })?;

        let mut out = Vec::new();
        for item in reps {
            let item_id = item.as_ref().ok_or_else(|| {
                IfcError::relationship("IfcProductRepresentation: item must be a reference")
            })?;
            let item_inst = self
                .file
                .instance(item_id)
                .ok_or_else(|| IfcError::unknown_instance(0, item_id))?;
            out.push(self.extract_item(item_inst)?);
        }
        Ok(out)
    }

    /// Extract geometry from a single representation item.
    pub fn extract_item(&self, item: &EntityInstance) -> Result<Geometry, IfcError> {
        match item.type_name.as_str() {
            "IFCFACETEDBREP" | "IFCADVANCEDBREP" => self.extract_brep(item),
            "IFCCSGSOLID" => self.extract_csg_solid(item),
            "IFCBOOLEANRESULT" | "IFCBOOLEANCLIPPINGRESULT" => {
                let node = self.extract_boolean_result(item)?;
                Ok(Geometry::Csg(node))
            }
            "IFCTRIANGULATEDFACESET" | "IFCPOLYGONALFACESET" => self.extract_tessellated(item),
            _other => Err(IfcError::Unsupported("unknown representation item")),
        }
    }

    // -----------------------------------------------------------------------
    // B-Rep extraction
    // -----------------------------------------------------------------------

    /// Extract a B-Rep (`IfcFacetedBrep`) into a mesh.
    ///
    /// `IfcFacetedBrep` = (Outer: IfcClosedShell). Each shell contains
    /// `IfcFace`s; each face has bounds (`IfcFaceOuterBound` /
    /// `IfcFaceBound`) referencing an `IfcPolyLoop` of `IfcCartesianPoint`s.
    fn extract_brep(&self, item: &EntityInstance) -> Result<Geometry, IfcError> {
        let mut mesh = Mesh::new();
        let outer = item.arg(0).and_then(IfcValue::as_ref).ok_or_else(|| {
            IfcError::relationship("IfcFacetedBrep: Outer (arg 0) must be a reference")
        })?;
        let shell =
            self.file.instance(outer).ok_or_else(|| IfcError::unknown_instance(0, outer))?;
        let faces_val = shell.arg(0).and_then(IfcValue::as_list).ok_or_else(|| {
            IfcError::relationship("IfcClosedShell: CfsFaces (arg 0) must be a list")
        })?;

        for face_val in faces_val {
            let face_id = face_val.as_ref().ok_or_else(|| {
                IfcError::relationship("IfcClosedShell: face must be a reference")
            })?;
            let face = self
                .file
                .instance(face_id)
                .ok_or_else(|| IfcError::unknown_instance(0, face_id))?;
            self.extract_face(face, &mut mesh)?;
        }
        Ok(Geometry::BRep(mesh))
    }

    /// Extract a single face into the mesh.
    fn extract_face(&self, face: &EntityInstance, mesh: &mut Mesh) -> Result<(), IfcError> {
        let bounds = face
            .arg(0)
            .and_then(IfcValue::as_list)
            .ok_or_else(|| IfcError::relationship("IfcFace: Bounds (arg 0) must be a list"))?;
        for bound_val in bounds {
            let bound_id = bound_val
                .as_ref()
                .ok_or_else(|| IfcError::relationship("IfcFace: bound must be a reference"))?;
            let bound = self
                .file
                .instance(bound_id)
                .ok_or_else(|| IfcError::unknown_instance(0, bound_id))?;
            self.extract_bound(bound, mesh)?;
        }
        Ok(())
    }

    /// Extract a face bound (a poly-loop) into the mesh.
    fn extract_bound(&self, bound: &EntityInstance, mesh: &mut Mesh) -> Result<(), IfcError> {
        let loop_ref = bound.arg(0).and_then(IfcValue::as_ref).ok_or_else(|| {
            IfcError::relationship("IfcFaceBound: Bound (arg 0) must be a reference")
        })?;
        let poly_loop =
            self.file.instance(loop_ref).ok_or_else(|| IfcError::unknown_instance(0, loop_ref))?;
        let points = poly_loop
            .arg(0)
            .and_then(IfcValue::as_list)
            .ok_or_else(|| IfcError::relationship("IfcPolyLoop: Polygon (arg 0) must be a list"))?;

        let base = mesh.vertices.len();
        let mut face_indices = Vec::with_capacity(points.len());
        for pt_val in points {
            let pt_id = pt_val
                .as_ref()
                .ok_or_else(|| IfcError::relationship("IfcPolyLoop: point must be a reference"))?;
            let pt =
                self.file.instance(pt_id).ok_or_else(|| IfcError::unknown_instance(0, pt_id))?;
            let coords = pt.arg(0).and_then(IfcValue::as_list).ok_or_else(|| {
                IfcError::relationship("IfcCartesianPoint: Coordinates (arg 0) must be a list")
            })?;
            let p = self.parse_point(coords, &pt_id)?;
            mesh.vertices.push(p);
            face_indices.push(base + face_indices.len());
        }
        mesh.faces.push(face_indices);
        Ok(())
    }

    /// Parse an `IfcCartesianPoint` coordinate list into a [`Point3`].
    fn parse_point(&self, coords: &[IfcValue], id: &usize) -> Result<Point3, IfcError> {
        let get = |i: usize| {
            coords.get(i).and_then(IfcValue::as_real).ok_or_else(|| {
                IfcError::relationship(format!(
                    "IfcCartesianPoint #{id}: coordinate {i} must be a real"
                ))
            })
        };
        Ok((get(0)?, get(1)?, get(2)?))
    }

    // -----------------------------------------------------------------------
    // CSG extraction
    // -----------------------------------------------------------------------

    /// Extract a CSG solid (`IfcCsgSolid`) into a [`CsgNode`].
    ///
    /// `IfcCsgSolid` = (TreeRootExpression).
    fn extract_csg_solid(&self, item: &EntityInstance) -> Result<Geometry, IfcError> {
        let root = item.arg(0).and_then(IfcValue::as_ref).ok_or_else(|| {
            IfcError::relationship("IfcCsgSolid: TreeRootExpression (arg 0) must be a reference")
        })?;
        let node = self.csg_node(root)?;
        Ok(Geometry::Csg(node))
    }

    /// Resolve a CSG node from an instance id.
    pub fn csg_node(&self, id: usize) -> Result<CsgNode, IfcError> {
        let inst = self.file.instance(id).ok_or_else(|| IfcError::unknown_instance(0, id))?;
        match inst.type_name.as_str() {
            "IFCBOOLEANRESULT" | "IFCBOOLEANCLIPPINGRESULT" => self.extract_boolean_result(inst),
            other => Ok(CsgNode::Primitive { kind: other.to_string(), id }),
        }
    }

    /// Extract a boolean result into a [`CsgNode`].
    ///
    /// `IfcBooleanResult` = (Operator, FirstOperand, SecondOperand).
    fn extract_boolean_result(&self, inst: &EntityInstance) -> Result<CsgNode, IfcError> {
        let operator = inst.arg(0).and_then(IfcValue::as_string).ok_or_else(|| {
            IfcError::relationship("IfcBooleanResult: Operator (arg 0) must be a string")
        })?;
        let first = inst.arg(1).and_then(IfcValue::as_ref).ok_or_else(|| {
            IfcError::relationship("IfcBooleanResult: FirstOperand (arg 1) must be a reference")
        })?;
        let second = inst.arg(2).and_then(IfcValue::as_ref).ok_or_else(|| {
            IfcError::relationship("IfcBooleanResult: SecondOperand (arg 2) must be a reference")
        })?;
        let left = Box::new(self.csg_node(first)?);
        let right = Box::new(self.csg_node(second)?);
        match operator {
            "UNION" => Ok(CsgNode::Union { left, right }),
            "INTERSECTION" => Ok(CsgNode::Intersection { left, right }),
            "DIFFERENCE" => Ok(CsgNode::Difference { left, right }),
            other => Err(IfcError::relationship(format!("unknown boolean operator: {other}"))),
        }
    }

    // -----------------------------------------------------------------------
    // Tessellated extraction
    // -----------------------------------------------------------------------

    /// Extract a tessellated face set into a mesh.
    ///
    /// `IfcTriangulatedFaceSet` / `IfcPolygonalFaceSet`:
    /// (Coordinates, Normals, Closed, CoordIndex, …).
    fn extract_tessellated(&self, item: &EntityInstance) -> Result<Geometry, IfcError> {
        let coord_inst = item.arg(0).and_then(IfcValue::as_ref).ok_or_else(|| {
            IfcError::relationship(
                "IfcTriangulatedFaceSet: Coordinates (arg 0) must be a reference",
            )
        })?;
        let coord = self
            .file
            .instance(coord_inst)
            .ok_or_else(|| IfcError::unknown_instance(0, coord_inst))?;
        let points_list = coord.arg(0).and_then(IfcValue::as_list).ok_or_else(|| {
            IfcError::relationship("IfcCartesianPointList3D: CoordList (arg 0) must be a list")
        })?;

        let mut vertices = Vec::with_capacity(points_list.len());
        for point_val in points_list {
            let point_list = point_val.as_list().ok_or_else(|| {
                IfcError::relationship("IfcCartesianPointList3D: coord must be a list of reals")
            })?;
            vertices.push(self.parse_point(point_list, &coord_inst)?);
        }

        let coord_index = item.arg(3).and_then(IfcValue::as_list).ok_or_else(|| {
            IfcError::relationship("IfcTriangulatedFaceSet: CoordIndex (arg 3) must be a list")
        })?;

        let mut faces = Vec::new();
        for face_val in coord_index {
            let idx_list = face_val.as_list().ok_or_else(|| {
                IfcError::relationship("IfcTriangulatedFaceSet: face index must be a list")
            })?;
            let mut face = Vec::with_capacity(idx_list.len());
            for i in idx_list {
                let idx = i.as_integer().ok_or_else(|| {
                    IfcError::relationship("IfcTriangulatedFaceSet: index must be an integer")
                })?;
                // IFC indices are 1-based.
                face.push((idx - 1) as usize);
            }
            faces.push(face);
        }
        Ok(Geometry::Tessellated(Mesh { vertices, faces }))
    }

    /// Return the raw file reference.
    pub fn file(&self) -> &'a IfcFile {
        self.file
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::IfcHeader;

    fn point_inst(id: usize, x: f64, y: f64, z: f64) -> EntityInstance {
        EntityInstance::new(
            id,
            "IFCCARTESIANPOINT",
            vec![IfcValue::List(vec![IfcValue::Real(x), IfcValue::Real(y), IfcValue::Real(z)])],
        )
    }

    #[test]
    fn extracts_tessellated_mesh() {
        // #1 IfcCartesianPointList3D, #2 IfcTriangulatedFaceSet
        let instances = vec![
            EntityInstance::new(
                1,
                "IFCCARTESIANPOINTLIST3D",
                vec![IfcValue::List(vec![
                    IfcValue::List(vec![
                        IfcValue::Real(0.0),
                        IfcValue::Real(0.0),
                        IfcValue::Real(0.0),
                    ]),
                    IfcValue::List(vec![
                        IfcValue::Real(1.0),
                        IfcValue::Real(0.0),
                        IfcValue::Real(0.0),
                    ]),
                    IfcValue::List(vec![
                        IfcValue::Real(0.0),
                        IfcValue::Real(1.0),
                        IfcValue::Real(0.0),
                    ]),
                    IfcValue::List(vec![
                        IfcValue::Real(0.0),
                        IfcValue::Real(0.0),
                        IfcValue::Real(1.0),
                    ]),
                ])],
            ),
            EntityInstance::new(
                2,
                "IFCTRIANGULATEDFACESET",
                vec![
                    IfcValue::Ref(1),
                    IfcValue::Derived,
                    IfcValue::Bool(false),
                    IfcValue::List(vec![
                        IfcValue::List(vec![
                            IfcValue::Integer(1),
                            IfcValue::Integer(2),
                            IfcValue::Integer(3),
                        ]),
                        IfcValue::List(vec![
                            IfcValue::Integer(1),
                            IfcValue::Integer(3),
                            IfcValue::Integer(4),
                        ]),
                    ]),
                ],
            ),
        ];
        let file = IfcFile { header: IfcHeader::default(), instances };
        let extractor = GeometryExtractor::new(&file);
        let geom = extractor.extract_item(&file.instances[1]).unwrap();
        match geom {
            Geometry::Tessellated(mesh) => {
                assert_eq!(mesh.vertex_count(), 4);
                assert_eq!(mesh.face_count(), 2);
            }
            other => panic!("expected tessellated, got {other:?}"),
        }
    }

    #[test]
    fn extracts_brep_mesh() {
        // Build a minimal closed shell with one quad face.
        // #10..13 points, #20 polyloop, #30 bound, #40 face, #50 shell, #60 brep
        let instances = vec![
            point_inst(10, 0.0, 0.0, 0.0),
            point_inst(11, 1.0, 0.0, 0.0),
            point_inst(12, 1.0, 1.0, 0.0),
            point_inst(13, 0.0, 1.0, 0.0),
            EntityInstance::new(
                20,
                "IFCPOLYLOOP",
                vec![IfcValue::List(vec![
                    IfcValue::Ref(10),
                    IfcValue::Ref(11),
                    IfcValue::Ref(12),
                    IfcValue::Ref(13),
                ])],
            ),
            EntityInstance::new(
                30,
                "IFCFACEOUTERBOUND",
                vec![IfcValue::Ref(20), IfcValue::Bool(true)],
            ),
            EntityInstance::new(40, "IFCFACE", vec![IfcValue::List(vec![IfcValue::Ref(30)])]),
            EntityInstance::new(
                50,
                "IFCCLOSEDSHELL",
                vec![IfcValue::List(vec![IfcValue::Ref(40)])],
            ),
            EntityInstance::new(60, "IFCFACETEDBREP", vec![IfcValue::Ref(50)]),
        ];

        let file = IfcFile { header: IfcHeader::default(), instances };
        let extractor = GeometryExtractor::new(&file);
        // instances[8] is the IFCFACETEDBREP.
        let geom = extractor.extract_item(&file.instances[8]).unwrap();
        match geom {
            Geometry::BRep(mesh) => {
                assert_eq!(mesh.vertex_count(), 4);
                assert_eq!(mesh.face_count(), 1);
                assert_eq!(mesh.faces[0], vec![0, 1, 2, 3]);
            }
            other => panic!("expected brep, got {other:?}"),
        }
    }

    #[test]
    fn extracts_csg_tree() {
        // #1 block primitive, #2 cylinder primitive, #3 boolean difference
        let instances = vec![
            EntityInstance::new(1, "IFCBLOCK", vec![IfcValue::Derived]),
            EntityInstance::new(2, "IFCCYLINDER", vec![IfcValue::Derived]),
            EntityInstance::new(
                3,
                "IFCBOOLEANRESULT",
                vec![
                    IfcValue::String("DIFFERENCE".to_string()),
                    IfcValue::Ref(1),
                    IfcValue::Ref(2),
                ],
            ),
            EntityInstance::new(4, "IFCCSGSOLID", vec![IfcValue::Ref(3)]),
        ];
        let file = IfcFile { header: IfcHeader::default(), instances };
        let extractor = GeometryExtractor::new(&file);
        let geom = extractor.extract_item(&file.instances[3]).unwrap();
        match geom {
            Geometry::Csg(CsgNode::Difference { left, right }) => {
                assert!(matches!(*left, CsgNode::Primitive { kind, .. } if kind == "IFCBLOCK"));
                assert!(matches!(*right, CsgNode::Primitive { kind, .. } if kind == "IFCCYLINDER"));
            }
            other => panic!("expected CSG difference, got {other:?}"),
        }
    }

    #[test]
    fn unsupported_item_type_errors() {
        let instances = vec![EntityInstance::new(1, "IFCSWEPTDISKSOLID", vec![IfcValue::Derived])];
        let file = IfcFile { header: IfcHeader::default(), instances };
        let extractor = GeometryExtractor::new(&file);
        assert!(matches!(
            extractor.extract_item(&file.instances[0]),
            Err(IfcError::Unsupported(_))
        ));
    }
}
