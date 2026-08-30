//! Model validation and consistency checking.
//!
//! Implements the syntactic/semantic split from `tpt-io-core` [`Validator`]:
//! [`SbmlParser::validate_syntactic`] checks structural correctness (identifiers,
//! references to compartments/species, MathML identifier resolution), while
//! [`SbmlParser::validate_semantic`] checks meaning (charge-balance consistency
//! and package reference integrity). [`validate_sbml`] collects *all* issues into
//! a [`ValidationReport`] so callers can surface every problem at once.

use alloc::string::{String, ToString};
use alloc::vec::Vec;

use tpt_io_core::{Validator, ValidationSettings};

use crate::error::SbmlError;
use crate::mathml::MathNode;
use crate::model::*;
use crate::parser::SbmlParser;

/// A single validation problem.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationIssue {
    /// SBML Level is not 2 or 3.
    UnsupportedLevel(u32),
    /// A required `id` was absent on an element that needs one.
    MissingId {
        /// Element kind (e.g. `"species"`).
        kind: &'static str,
        /// Human-readable context.
        context: String,
    },
    /// A duplicate identifier was found.
    DuplicateId {
        /// Element kind.
        kind: &'static str,
        /// The offending identifier.
        id: String,
    },
    /// A species references a non-existent compartment.
    UnknownCompartment {
        /// Species id.
        species: String,
        /// Compartment id that was referenced.
        compartment: String,
    },
    /// A reaction references a non-existent species in a given role.
    UnknownReactionSpecies {
        /// Reaction id.
        reaction: String,
        /// Role: `"reactant"`, `"product"`, or `"modifier"`.
        role: &'static str,
        /// Species id that was referenced.
        species: String,
    },
    /// A MathML `ci` referenced an unknown identifier.
    UnknownReference {
        /// Context describing where the expression appeared.
        context: String,
        /// The unknown identifier.
        id: String,
    },
    /// A reaction is not charge-balanced (net charge of products minus reactants).
    ChargeImbalance {
        /// Reaction id.
        reaction: String,
        /// Net charge (products − reactants).
        net: i64,
    },
    /// An FBC flux bound references an unknown reaction.
    FluxBoundUnknownReaction {
        /// Flux bound id (or `"?"`).
        flux_bound: String,
        /// Reaction id that was referenced.
        reaction: String,
    },
    /// An FBC objective references an unknown reaction.
    ObjectiveUnknownReaction {
        /// Objective id.
        objective: String,
        /// Reaction id that was referenced.
        reaction: String,
    },
}

impl core::fmt::Display for ValidationIssue {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            ValidationIssue::UnsupportedLevel(l) => write!(f, "unsupported SBML level {l} (only 2 and 3 are supported)"),
            ValidationIssue::MissingId { kind, context } => write!(f, "<{kind}> is missing a required id ({context})"),
            ValidationIssue::DuplicateId { kind, id } => write!(f, "duplicate {kind} identifier '{id}'"),
            ValidationIssue::UnknownCompartment { species, compartment } => {
                write!(f, "species '{species}' references unknown compartment '{compartment}'")
            }
            ValidationIssue::UnknownReactionSpecies { reaction, role, species } => {
                write!(f, "reaction '{reaction}' {role} references unknown species '{species}'")
            }
            ValidationIssue::UnknownReference { context, id } => write!(f, "{context} references unknown identifier '{id}'"),
            ValidationIssue::ChargeImbalance { reaction, net } => {
                write!(f, "reaction '{reaction}' is not charge-balanced (net charge {net})")
            }
            ValidationIssue::FluxBoundUnknownReaction { flux_bound, reaction } => {
                write!(f, "flux bound '{flux_bound}' references unknown reaction '{reaction}'")
            }
            ValidationIssue::ObjectiveUnknownReaction { objective, reaction } => {
                write!(f, "objective '{objective}' references unknown reaction '{reaction}'")
            }
        }
    }
}

/// Aggregate result of validating a model: errors (must-fix) and warnings.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ValidationReport {
    /// Errors that violate SBML structural/semantic rules.
    pub errors: Vec<ValidationIssue>,
    /// Non-fatal warnings.
    pub warnings: Vec<ValidationIssue>,
}

impl ValidationReport {
    /// Whether any errors were recorded.
    pub fn is_ok(&self) -> bool {
        self.errors.is_empty()
    }

    /// Convert the first error (if any) into a [`SbmlError`].
    pub fn into_result(self) -> Result<(), SbmlError> {
        if let Some(first) = self.errors.into_iter().next() {
            return Err(SbmlError::Validation(first.to_string()));
        }
        Ok(())
    }
}

fn species_charge(s: &Species) -> Option<i64> {
    s.fbc_charge.or(s.charge)
}

fn collect_refs(math: &MathNode, local_scope: &[String], known: &[String], context: &str, out: &mut Vec<ValidationIssue>) {
    let mut refs = Vec::new();
    math.collect_references(&mut refs);
    for r in refs {
        if known.iter().any(|k| k == &r) || local_scope.iter().any(|k| k == &r) {
            continue;
        }
        out.push(ValidationIssue::UnknownReference { context: context.to_string(), id: r });
    }
}

/// Structural (syntactic) checks.
pub fn check_syntactic(sbml: &Sbml) -> Vec<ValidationIssue> {
    let mut issues = Vec::new();
    if sbml.level != 2 && sbml.level != 3 {
        issues.push(ValidationIssue::UnsupportedLevel(sbml.level));
    }

    let m = &sbml.model;
    let known = m.known_identifiers();

    // Duplicate id checks across primary collections.
    let mut seen = Vec::new();
    for c in &m.compartments {
        if seen.contains(&c.id) {
            issues.push(ValidationIssue::DuplicateId { kind: "compartment", id: c.id.clone() });
        }
        seen.push(c.id.clone());
    }
    for s in &m.species {
        if seen.contains(&s.id) {
            issues.push(ValidationIssue::DuplicateId { kind: "species", id: s.id.clone() });
        }
        seen.push(s.id.clone());
    }
    for p in &m.parameters {
        if seen.contains(&p.id) {
            issues.push(ValidationIssue::DuplicateId { kind: "parameter", id: p.id.clone() });
        }
        seen.push(p.id.clone());
    }
    for r in &m.reactions {
        if seen.contains(&r.id) {
            issues.push(ValidationIssue::DuplicateId { kind: "reaction", id: r.id.clone() });
        }
        seen.push(r.id.clone());
    }
    for f in &m.function_definitions {
        if seen.contains(&f.id) {
            issues.push(ValidationIssue::DuplicateId { kind: "functionDefinition", id: f.id.clone() });
        }
        seen.push(f.id.clone());
    }
    for u in &m.unit_definitions {
        if seen.contains(&u.id) {
            issues.push(ValidationIssue::DuplicateId { kind: "unitDefinition", id: u.id.clone() });
        }
        seen.push(u.id.clone());
    }

    // Species must reference an existing compartment.
    for s in &m.species {
        if m.compartment_by_id(&s.compartment).is_none() {
            issues.push(ValidationIssue::UnknownCompartment { species: s.id.clone(), compartment: s.compartment.clone() });
        }
    }

    // Reaction species references must resolve.
    for r in &m.reactions {
        for sr in r.reactants.iter().chain(r.products.iter()) {
            if m.species_by_id(&sr.species).is_none() {
                issues.push(ValidationIssue::UnknownReactionSpecies { reaction: r.id.clone(), role: "reactant/product", species: sr.species.clone() });
            }
        }
        for sr in &r.modifiers {
            if m.species_by_id(&sr.species).is_none() {
                issues.push(ValidationIssue::UnknownReactionSpecies { reaction: r.id.clone(), role: "modifier", species: sr.species.clone() });
            }
        }
    }

    // MathML reference resolution.
    for (i, ia) in m.initial_assignments.iter().enumerate() {
        let ctx = format!("initialAssignment[{}] ({})", i, ia.symbol);
        collect_refs(&ia.math, &[], &known, &ctx, &mut issues);
    }
    for (i, rule) in m.rules.iter().enumerate() {
        let (ctx, math, scope) = match rule {
            Rule::Assignment { variable, math } => (format!("assignmentRule ({variable})"), math, Vec::new()),
            Rule::Rate { variable, math } => (format!("rateRule ({variable})"), math, Vec::new()),
            Rule::Algebraic { variable, math } => (format!("algebraicRule ({})", variable.as_deref().unwrap_or("?")), math, Vec::new()),
        };
        let _ = i;
        collect_refs(math, &scope, &known, &ctx, &mut issues);
    }
    for (i, con) in m.constraints.iter().enumerate() {
        collect_refs(&con.math, &[], &known, &format!("constraint[{i}]"), &mut issues);
    }
    for (i, r) in m.reactions.iter().enumerate() {
        if let Some(kl) = &r.kinetic_law {
            let scope: Vec<String> = kl.parameters.iter().map(|p| p.id.clone()).collect();
            if let Some(math) = &kl.math {
                collect_refs(math, &scope, &known, &format!("reaction '{}' kineticLaw", r.id), &mut issues);
            }
            let _ = i;
        }
    }
    for (i, ev) in m.events.iter().enumerate() {
        if let Some(trig) = &ev.trigger {
            collect_refs(&trig.math, &[], &known, &format!("event[{}] trigger", i), &mut issues);
        }
        if let Some(d) = &ev.delay {
            collect_refs(d, &[], &known, &format!("event[{}] delay", i), &mut issues);
        }
        for ea in &ev.event_assignments {
            collect_refs(&ea.math, &[], &known, &format!("event[{}] assignment ({})", i, ea.variable), &mut issues);
        }
    }
    for fd in &m.function_definitions {
        // Function body is a lambda; bvars are the local scope.
        let mut bvars = Vec::new();
        if let MathNode::Lambda { bvars: bs, .. } = &fd.math {
            bvars.extend(bs.iter().cloned());
        }
        collect_refs(&fd.math, &bvars, &known, &format!("functionDefinition '{}'", fd.id), &mut issues);
    }

    issues
}

/// Semantic / consistency checks (meaning, not just structure).
pub fn check_semantic(sbml: &Sbml) -> Vec<ValidationIssue> {
    let mut issues = Vec::new();
    let m = &sbml.model;

    // Charge balance per reaction.
    for r in &m.reactions {
        let mut net: i64 = 0;
        let mut have_charge = true;
        for sr in &r.products {
            match m.species_by_id(&sr.species).and_then(species_charge) {
                Some(c) => net += (c as f64 * sr.stoichiometry).round() as i64,
                None => have_charge = false,
            }
        }
        for sr in &r.reactants {
            match m.species_by_id(&sr.species).and_then(species_charge) {
                Some(c) => net -= (c as f64 * sr.stoichiometry).round() as i64,
                None => have_charge = false,
            }
        }
        if have_charge && net != 0 {
            issues.push(ValidationIssue::ChargeImbalance { reaction: r.id.clone(), net });
        }
    }

    // FBC reference integrity.
    if let Some(fbc) = &m.fbc {
        for fb in &fbc.flux_bounds {
            if m.reaction_by_id(&fb.reaction).is_none() {
                issues.push(ValidationIssue::FluxBoundUnknownReaction {
                    flux_bound: fb.id.clone().unwrap_or_else(|| "?".to_string()),
                    reaction: fb.reaction.clone(),
                });
            }
        }
        for o in &fbc.objectives {
            for fo in &o.flux_objectives {
                if m.reaction_by_id(&fo.reaction).is_none() {
                    issues.push(ValidationIssue::ObjectiveUnknownReaction { objective: o.id.clone(), reaction: fo.reaction.clone() });
                }
            }
        }
    }

    issues
}

/// Run both syntactic and semantic checks, returning a full report.
pub fn validate_sbml(sbml: &Sbml) -> ValidationReport {
    let mut report = ValidationReport::default();
    report.errors.extend(check_syntactic(sbml));
    report.errors.extend(check_semantic(sbml));
    report
}

impl SbmlParser {
    /// Validate a parsed model, returning the full issue report.
    pub fn validate(&self, sbml: &Sbml) -> ValidationReport {
        validate_sbml(sbml)
    }
}

impl Validator<Sbml> for SbmlParser {
    type Error = SbmlError;

    fn validate_syntactic(&self, value: &Sbml) -> Result<(), SbmlError> {
        let mut report = ValidationReport::default();
        report.errors = check_syntactic(value);
        report.into_result()
    }

    fn validate_semantic(&self, value: &Sbml) -> Result<(), SbmlError> {
        let mut report = ValidationReport::default();
        report.errors = check_semantic(value);
        report.into_result()
    }
}

/// Convenience: run validation honouring the parser's [`ValidationSettings`].
impl SbmlParser {
    /// Validate a model according to the configured [`ValidationSettings`].
    pub fn validate_with_settings(&self, sbml: &Sbml, settings: ValidationSettings) -> ValidationReport {
        let mut report = ValidationReport::default();
        if settings.is_syntactic() {
            report.errors.extend(check_syntactic(sbml));
        }
        if settings.is_semantic() {
            report.errors.extend(check_semantic(sbml));
        }
        report
    }
}
