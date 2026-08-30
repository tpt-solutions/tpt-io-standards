# tpt-io-ifc

IFC (Industry Foundation Classes) 4.0 / 4.3 parser for the `tpt-io` workspace,
targeting Building Information Modeling (BIM) data.

The crate implements a from-scratch, pure-Rust parser for the STEP-Physical File
(SPF / ISO 10303-21) encoding of IFC building data. It tokenizes and parses the
file into a structured [`IfcFile`], resolves entity instances and their
references, and provides higher-level traversals: relationship graph
construction, property set traversal, geometry extraction (B-Reps, CSG, and
tessellated meshes), schema version detection and migration, spatial structure
navigation, and round-trip serialization.

[`IfcFile`]: https://docs.rs/tpt-io-ifc/latest/tpt_io_ifc/model/struct.IfcFile.html

## Quick start

```rust
use tpt_io_core::Reader;
use tpt_io_ifc::parser::IfcReader;
use tpt_io_ifc::model::IfcFile;

let src = b"ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION(('ViewDefinition'),'2;1');\nFILE_NAME('t.ifc','2024',(),(),'','','');\nFILE_SCHEMA(('IFC4'));\nENDSEC;\nDATA;\n#1=IFCWALL('g',$,'Wall',$,$,$,$,$);\nENDSEC;\nEND-ISO-10303-21;\n";
let file = IfcReader::new().parse(src).unwrap();
assert_eq!(file.instance_count(), 1);
assert_eq!(file.schema(), Some("IFC4"));
```

## What is supported

- **SPF tokenization & parsing**: full ISO 10303-21 lexical grammar (keywords,
  instance references, enumerations, strings with `''` escaping, integers,
  reals, nested aggregates, line and nested block comments) and document
  structure (`ISO-10303-21`, `HEADER`, `DATA`, `ENDSEC`, `END-ISO-10303-21`).
- **Entity instance resolution**: every `#id = TYPE(args);` is parsed into a
  strongly typed [`EntityInstance`], with dangling-reference detection.
- **Relationship graph**: index and traverse `IfcRelContainedInSpatialStructure`,
  `IfcRelAssociatesMaterial`, `IfcRelDefinesByProperties`, and
  `IfcRelAggregates` via [`RelGraph`].
- **Property set traversal**: resolve `IfcPropertySet` → `IfcPropertySingleValue`
  / `IfcPropertyBoundedValue` / `IfcPropertyListValue` /
  `IfcPropertyReferenceValue` / `IfcPropertyTableValue` chains linked to
  elements.
- **Geometry extraction**: reduce B-Reps ([`IfcFacetedBrep`]), CSG solids
  ([`IfcCsgSolid`], [`IfcBooleanResult`]), and tessellated face sets
  ([`IfcTriangulatedFaceSet`], [`IfcPolygonalFaceSet`]) to a neutral mesh / CSG
  tree via [`GeometryExtractor`].
- **Schema version detection & migration**: detect `IFC2X3` / `IFC4` / `IFC4X3`
  from the `FILE_SCHEMA` header and map entity/property-set names between
  versions.
- **Spatial navigation**: walk the decomposition tree Project → Site → Building
  → Storey → Space and query containment.
- **Round-trip serialization**: serialize a parsed [`IfcFile`] back to SPF text;
  re-parsing yields a structurally identical file.

[`EntityInstance`]: https://docs.rs/tpt-io-ifc/latest/tpt_io_ifc/model/struct.EntityInstance.html
[`RelGraph`]: https://docs.rs/tpt-io-ifc/latest/tpt_io_ifc/graph/struct.RelGraph.html
[`GeometryExtractor`]: https://docs.rs/tpt-io-ifc/latest/tpt_io_ifc/geometry/struct.GeometryExtractor.html
[`IfcFacetedBrep`]: https://standards.buildingsmart.org/IFC/RELEASE/IFC4/ADD2_TC1/HTML/schema/ifcgeometricmodelresource/lexical/ifcfacetedbrep.htm
[`IfcCsgSolid`]: https://standards.buildingsmart.org/IFC/RELEASE/IFC4/ADD2_TC1/HTML/schema/ifcgeometricmodelresource/lexical/ifccsgsolid.htm
[`IfcBooleanResult`]: https://standards.buildingsmart.org/IFC/RELEASE/IFC4/ADD2_TC1/HTML/schema/ifcgeometricmodelresource/lexical/ifcbooleanresult.htm
[`IfcTriangulatedFaceSet`]: https://standards.buildingsmart.org/IFC/RELEASE/IFC4/ADD2_TC1/HTML/schema/ifctopologyresource/lexical/ifctriangulatedfaceset.htm
[`IfcPolygonalFaceSet`]: https://standards.buildingsmart.org/IFC/RELEASE/IFC4/ADD2_TC1/HTML/schema/ifctopologyresource/lexical/ifcpolygonalfaceset.htm

## Validation

The [`IfcValidator`] runs a syntactic pass (unique instance ids; a declared
schema) and a semantic pass (the DATA section is non-empty). Disable either via
[`tpt_io_core::ValidationSettings`].

[`IfcValidator`]: https://docs.rs/tpt-io-ifc/latest/tpt_io_ifc/parser/struct.IfcValidator.html
[`tpt_io_core::ValidationSettings`]: https://docs.rs/tpt-io-core/latest/tpt_io_core/struct.ValidationSettings.html

## Features

| Feature | Default | Effect |
| --- | --- | --- |
| `std` | yes | `std`-only conveniences |
| `alloc` | no | `alloc`-based types without `std` |
| `parallel` | no | Parallel entity-instance parsing via Rayon (spec §4 Performance) |
| `eng-geometry-integration` | no | Stub conversion into a future `tpt-eng-geometry` geometry |
| `eng-cad-integration` | no | Stub conversion into a future `tpt-eng-cad` CAD model |

The `eng-*` features are integration stubs: they compile and link today so the
feature flags are stable, but the downstream `tpt-eng-*` crates are not yet
published. Only the conversion bodies will need to change when they land.

## License

Licensed under either of [MIT](../LICENSE-MIT) or [Apache-2.0](../LICENSE-APACHE)
at your option.
