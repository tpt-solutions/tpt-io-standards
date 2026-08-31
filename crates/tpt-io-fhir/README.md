# tpt-io-fhir

HL7 FHIR R4/R5 deserializer for the `tpt-io` workspace — JSON and XML
representations, profile validation, NZ HIPC 2020 compliance, reference
resolution, bundle traversal, FHIRPath evaluation, and batch import/export.

## Quick start

```rust
use tpt_io_fhir::{json, xml, Resource};

// Parse from JSON
let r: Resource = json::parse_resource(r#"{
    "resourceType": "Patient",
    "id": "example",
    "name": [{"family": "Chalmers", "given": ["Peter"]}]
}"#)?;

// Parse from XML
let r: Resource = xml::parse_resource(r#"<Patient xmlns="http://hl7.org/fhir">
    <id value="example"/>
    <name><family value="Chalmers"/><given value="Peter"/></name>
</Patient>"#)?;

// FHIRPath evaluation
use tpt_io_fhir::fhirpath::evaluate_resource;
let given = evaluate_resource("name.given.first()", PATIENT_JSON)?;
```

## Features

| Feature | Default | Effect |
| --- | --- | --- |
| `std` | yes | `std`-only conveniences |
| `alloc` | no | `alloc`-based types without `std` |
| `json` | yes | JSON deserialization via `serde_json` (implies `alloc`). |
| `xml` | yes | XML deserialization via `quick-xml` (implies `std`). |
| `validation` | yes | Profile + NZ HIPC 2020 compliance validation (implies `alloc`). |
| `reference` | yes | Cross-resource reference resolution (implies `alloc`). |
| `bundle` | yes | Bundle traversal + pagination helpers (implies `alloc`). |
| `fhirpath` | yes | FHIRPath expression evaluator (implies `alloc`). |
| `batch` | yes | Batch import/export utilities (implies `alloc`). |
| `medical-patient-integration` | no | Stub conversion to a future `tpt-medical-patient` crate. |

## Testing

```sh
cargo test -p tpt-io-fhir
cargo +nightly fuzz run fhir
```

## Status

Per the workspace `todo.md`, `tpt-io-fhir` implements JSON and XML
deserialization for Patient, Observation, and Bundle (with a generic
`Resource::Other` fallback for unmodeled kinds), strict profile validation,
NZ HIPC 2020 compliance hooks, cross-resource reference resolution, bundle
traversal with pagination, a FHIRPath expression evaluator, and batch
builders. Round-trip, property-based, and golden-file tests are included.
`cargo publish --dry-run` is blocked until `tpt-io-core` is on crates.io
(see §4 publish order).

## License

Licensed under either of [MIT](../LICENSE-MIT) or [Apache-2.0](../LICENSE-APACHE)
at your option.
