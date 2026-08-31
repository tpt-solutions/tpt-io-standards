//! Integration tests: golden-file corpus covering the core model and the FBC,
//! Qual, and Spatial package extensions.
//!
//! Fixtures live in `tests/samples/` and are hand-written, valid SBML documents
//! that independently exercise each parser path.

use std::path::PathBuf;

use tpt_io_core::{Reader, Validator};
use tpt_io_sbml::{parse_sbml, Sbml, SbmlParser, ValidationIssue};

fn sample(name: &str) -> Vec<u8> {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("tests");
    p.push("samples");
    p.push(name);
    std::fs::read(&p).unwrap_or_else(|e| panic!("failed to read {}: {e}", p.display()))
}

#[test]
fn golden_l2_core_features() {
    let doc = sample("biomodel_l2.xml");
    let sbml = Sbml::parse(&doc).expect("parse biomodel_l2.xml");

    assert_eq!(sbml.level, 2);
    assert_eq!(sbml.version, 4);
    assert_eq!(sbml.model.id.as_deref(), Some("branchModel"));
    assert_eq!(sbml.model.compartments.len(), 1);
    assert_eq!(sbml.model.species.len(), 2);
    assert_eq!(sbml.model.reactions.len(), 1);

    let r = &sbml.model.reactions[0];
    assert!(!r.reversible);
    assert_eq!(r.reactants[0].species, "S1");
    assert_eq!(r.products[0].species, "S2");
    let kl = r.kinetic_law.as_ref().unwrap();
    assert_eq!(kl.parameters[0].id, "k1");
    assert_eq!(kl.math.as_ref().unwrap().to_infix_string(), "k1 * S1");

    // Rule, event, and function definition paths.
    assert_eq!(sbml.model.rules.len(), 1);
    assert_eq!(sbml.model.events.len(), 1);
    let ev = &sbml.model.events[0];
    assert_eq!(ev.id.as_deref(), Some("E1"));
    assert!(ev.trigger.is_some());
    assert!(ev.delay.is_some());
    assert_eq!(ev.event_assignments.len(), 1);
    assert_eq!(sbml.model.function_definitions.len(), 1);
    let fd = &sbml.model.function_definitions[0];
    assert_eq!(fd.id, "f");
    // Lambda body: x^2.
    assert_eq!(fd.math.to_infix_string(), "lambda(x : x ^ 2)");

    // Validation must pass for this well-formed model.
    let report = tpt_io_sbml::validate_sbml(&sbml);
    assert!(report.is_ok(), "issues: {:?}", report.errors);
}

#[test]
fn golden_fbc_package() {
    let doc = sample("fbc_l3.xml");
    let sbml = Sbml::parse(&doc).expect("parse fbc_l3.xml");

    assert_eq!(sbml.level, 3);
    let fbc = sbml.model.fbc.as_ref().expect("fbc present");
    assert!(fbc.strict);
    assert_eq!(fbc.objectives.len(), 1);
    assert_eq!(fbc.objectives[0].id, "obj1");
    assert_eq!(fbc.objectives[0].type_, "maximize");
    assert_eq!(fbc.objectives[0].flux_objectives[0].reaction, "R_GLC");
    assert_eq!(fbc.flux_bounds.len(), 2);
    assert_eq!(fbc.flux_bounds[0].operation, "greaterEqual");
    assert_eq!(fbc.flux_bounds[0].value, 0.0);
    assert_eq!(fbc.gene_products.len(), 1);
    assert_eq!(fbc.gene_products[0].label.as_deref(), Some("gene1"));

    // Reaction-level FBC attributes.
    let r = &sbml.model.reactions[0];
    assert_eq!(r.lower_flux_bound.as_deref(), Some("lb"));
    assert_eq!(r.upper_flux_bound.as_deref(), Some("ub"));
    assert_eq!(r.gene_product_association.as_deref(), Some("( g1 )"));

    // Chemical formulas captured.
    let glc = sbml.model.species_by_id("Glc").unwrap();
    assert_eq!(glc.chemical_formula.as_deref(), Some("C6H12O6"));

    // Validation passes (references resolve; Glc -> 2 Pyr is unbalanced but we
    // only assert no structural/reference errors here, not element balance).
    let report = tpt_io_sbml::validate_sbml(&sbml);
    assert!(
        report.errors.iter().all(|e| !matches!(
            e,
            ValidationIssue::UnknownReactionSpecies { .. }
                | ValidationIssue::FluxBoundUnknownReaction { .. }
                | ValidationIssue::ObjectiveUnknownReaction { .. }
        )),
        "unexpected reference errors: {:?}",
        report.errors
    );
}

#[test]
fn golden_qual_package() {
    let doc = sample("qual_l3.xml");
    let sbml = Sbml::parse(&doc).expect("parse qual_l3.xml");

    let qual = sbml.model.qual.as_ref().expect("qual present");
    assert_eq!(qual.qualitative_species.len(), 2);
    assert_eq!(qual.qualitative_species[0].id, "q1");
    assert_eq!(qual.qualitative_species[0].initial_level, Some(1.0));
    assert_eq!(qual.transitions.len(), 1);
    let t = &qual.transitions[0];
    assert_eq!(t.inputs[0].qualitative_species, "q1");
    assert_eq!(t.inputs[0].sign.as_deref(), Some("positive"));
    assert_eq!(t.outputs[0].qualitative_species, "q2");
    assert_eq!(t.outputs[0].output_level, Some(1.0));
    assert!(t.function.is_some());
}

#[test]
fn golden_spatial_package() {
    let doc = sample("spatial_l3.xml");
    let sbml = Sbml::parse(&doc).expect("parse spatial_l3.xml");

    let spatial = sbml.model.spatial.as_ref().expect("spatial present");
    let geom = spatial.geometry.as_ref().expect("geometry present");
    assert_eq!(geom.id, "geom");
    assert_eq!(geom.dimensions, Some(3));
    assert_eq!(geom.coordinate_systems, ["cs1"]);
    assert_eq!(geom.domain_types, ["dt1"]);
    assert_eq!(geom.domains, ["d1"]);
    assert_eq!(geom.geometry_definitions, ["gd1"]);
}

#[test]
fn reader_trait_and_validator() {
    let doc = sample("biomodel_l2.xml");
    let parser = SbmlParser::default();
    let sbml: Sbml = parser.parse(&doc).expect("parsed via Reader");
    assert!(Validator::<Sbml>::validate_syntactic(&parser, &sbml).is_ok());
}

#[test]
fn unknown_input_does_not_panic_and_reports_error() {
    // Not SBML at all.
    let res = parse_sbml(b"not xml <<<");
    assert!(res.is_err());
}
