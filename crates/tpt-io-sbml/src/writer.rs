//! SBML serializer for the `tpt-io` workspace.
//!
//! Renders a [`model::Sbml`] back to a valid SBML Level 2/3 XML document.
//! Covers the core model elements and the FBC / Qual / Spatial package
//! extensions. MathML expressions are serialized as Content MathML.

use alloc::string::ToString;
use alloc::vec::Vec;

use quick_xml::events::{BytesDecl, BytesEnd, BytesStart, BytesText, Event};
use quick_xml::Writer;

use crate::error::SbmlError;
use crate::mathml::MathNode;
use crate::model::*;

/// Serialize an [`Sbml`] document to UTF-8 XML bytes.
pub fn serialize(sbml: &Sbml) -> Result<Vec<u8>, SbmlError> {
    let mut writer = Writer::new(Vec::new());
    let decl = BytesDecl::new("1.0", Some("UTF-8"), None);
    writer.write_event(Event::Decl(decl))?;
    write_sbml(&mut writer, sbml)?;
    Ok(writer.into_inner())
}

fn write_sbml(writer: &mut Writer<Vec<u8>>, sbml: &Sbml) -> Result<(), SbmlError> {
    let ns = if sbml.level == 3 {
        "http://www.sbml.org/sbml/level3/version2/core"
    } else {
        "http://www.sbml.org/sbml/level2/version4"
    };
    let mut sbml_start = BytesStart::new("sbml");
    sbml_start.extend_attributes([
        ("xmlns", ns),
        ("level", sbml.level.to_string().as_str()),
        ("version", sbml.version.to_string().as_str()),
    ]);
    if sbml.model.fbc.is_some() {
        sbml_start.extend_attributes([(
            "xmlns:fbc",
            "http://www.sbml.org/sbml/level3/version1/fbc/version2",
        )]);
    }
    if sbml.model.qual.is_some() {
        sbml_start.extend_attributes([(
            "xmlns:qual",
            "http://www.sbml.org/sbml/level3/version1/qual/version1",
        )]);
    }
    if sbml.model.spatial.is_some() {
        sbml_start.extend_attributes([(
            "xmlns:spatial",
            "http://www.sbml.org/sbml/level3/version1/spatial/version1",
        )]);
    }
    writer.write_event(Event::Start(sbml_start))?;
    write_model(writer, &sbml.model, sbml.level)?;
    writer.write_event(Event::End(BytesEnd::new("sbml")))?;
    Ok(())
}

fn write_model(writer: &mut Writer<Vec<u8>>, model: &Model, level: u32) -> Result<(), SbmlError> {
    let mut model_start = BytesStart::new("model");
    if let Some(id) = &model.id {
        model_start.extend_attributes([("id", id.as_str())]);
    }
    if let Some(name) = &model.name {
        model_start.extend_attributes([("name", name.as_str())]);
    }
    if let Some(fbc) = &model.fbc {
        model_start.extend_attributes([("fbc:strict", if fbc.strict { "true" } else { "false" })]);
    }
    writer.write_event(Event::Start(model_start))?;

    if !model.unit_definitions.is_empty() {
        writer.write_event(Event::Start(BytesStart::new("listOfUnitDefinitions")))?;
        for ud in &model.unit_definitions {
            write_unit_definition(writer, ud)?;
        }
        writer.write_event(Event::End(BytesEnd::new("listOfUnitDefinitions")))?;
    }

    if !model.compartments.is_empty() {
        writer.write_event(Event::Start(BytesStart::new("listOfCompartments")))?;
        for c in &model.compartments {
            write_compartment(writer, c, level)?;
        }
        writer.write_event(Event::End(BytesEnd::new("listOfCompartments")))?;
    }

    if !model.species.is_empty() {
        writer.write_event(Event::Start(BytesStart::new("listOfSpecies")))?;
        for s in &model.species {
            write_species(writer, s, level)?;
        }
        writer.write_event(Event::End(BytesEnd::new("listOfSpecies")))?;
    }

    if !model.parameters.is_empty() {
        writer.write_event(Event::Start(BytesStart::new("listOfParameters")))?;
        for p in &model.parameters {
            write_parameter(writer, p)?;
        }
        writer.write_event(Event::End(BytesEnd::new("listOfParameters")))?;
    }

    if !model.initial_assignments.is_empty() {
        writer.write_event(Event::Start(BytesStart::new("listOfInitialAssignments")))?;
        for ia in &model.initial_assignments {
            write_initial_assignment(writer, ia)?;
        }
        writer.write_event(Event::End(BytesEnd::new("listOfInitialAssignments")))?;
    }

    if !model.rules.is_empty() {
        writer.write_event(Event::Start(BytesStart::new("listOfRules")))?;
        for r in &model.rules {
            write_rule(writer, r)?;
        }
        writer.write_event(Event::End(BytesEnd::new("listOfRules")))?;
    }

    if !model.constraints.is_empty() {
        writer.write_event(Event::Start(BytesStart::new("listOfConstraints")))?;
        for c in &model.constraints {
            write_constraint(writer, c)?;
        }
        writer.write_event(Event::End(BytesEnd::new("listOfConstraints")))?;
    }

    if !model.reactions.is_empty() {
        writer.write_event(Event::Start(BytesStart::new("listOfReactions")))?;
        for r in &model.reactions {
            write_reaction(writer, r, level)?;
        }
        writer.write_event(Event::End(BytesEnd::new("listOfReactions")))?;
    }

    if !model.events.is_empty() {
        writer.write_event(Event::Start(BytesStart::new("listOfEvents")))?;
        for e in &model.events {
            write_event(writer, e, level)?;
        }
        writer.write_event(Event::End(BytesEnd::new("listOfEvents")))?;
    }

    if !model.function_definitions.is_empty() {
        writer.write_event(Event::Start(BytesStart::new("listOfFunctionDefinitions")))?;
        for fd in &model.function_definitions {
            write_function_definition(writer, fd)?;
        }
        writer.write_event(Event::End(BytesEnd::new("listOfFunctionDefinitions")))?;
    }

    if let Some(fbc) = &model.fbc {
        write_fbc_model(writer, fbc)?;
    }

    if let Some(qual) = &model.qual {
        write_qual_model(writer, qual)?;
    }

    if let Some(spatial) = &model.spatial {
        write_spatial_model(writer, spatial)?;
    }

    writer.write_event(Event::End(BytesEnd::new("model")))?;
    Ok(())
}

fn write_unit_definition(
    writer: &mut Writer<Vec<u8>>,
    ud: &UnitDefinition,
) -> Result<(), SbmlError> {
    let mut start = BytesStart::new("unitDefinition");
    start.extend_attributes([("id", ud.id.as_str())]);
    if let Some(name) = &ud.name {
        start.extend_attributes([("name", name.as_str())]);
    }
    writer.write_event(Event::Start(start))?;
    if !ud.units.is_empty() {
        writer.write_event(Event::Start(BytesStart::new("listOfUnits")))?;
        for u in &ud.units {
            write_unit(writer, u)?;
        }
        writer.write_event(Event::End(BytesEnd::new("listOfUnits")))?;
    }
    writer.write_event(Event::End(BytesEnd::new("unitDefinition")))?;
    Ok(())
}

fn write_unit(writer: &mut Writer<Vec<u8>>, u: &Unit) -> Result<(), SbmlError> {
    let mut start = BytesStart::new("unit");
    start.extend_attributes([("kind", u.kind.as_str())]);
    if u.exponent != 1.0 {
        start.extend_attributes([("exponent", u.exponent.to_string().as_str())]);
    }
    if u.scale != 0 {
        start.extend_attributes([("scale", u.scale.to_string().as_str())]);
    }
    if u.multiplier != 1.0 {
        start.extend_attributes([("multiplier", u.multiplier.to_string().as_str())]);
    }
    writer.write_event(Event::Empty(start))?;
    Ok(())
}

fn write_compartment(
    writer: &mut Writer<Vec<u8>>,
    c: &Compartment,
    _level: u32,
) -> Result<(), SbmlError> {
    let mut start = BytesStart::new("compartment");
    start.extend_attributes([("id", c.id.as_str())]);
    if let Some(name) = &c.name {
        start.extend_attributes([("name", name.as_str())]);
    }
    if let Some(spatial_dimensions) = c.spatial_dimensions {
        start.extend_attributes([("spatialDimensions", spatial_dimensions.to_string().as_str())]);
    }
    if let Some(size) = c.size {
        start.extend_attributes([("size", size.to_string().as_str())]);
    }
    if let Some(units) = &c.units {
        start.extend_attributes([("units", units.as_str())]);
    }
    if !c.constant {
        start.extend_attributes([("constant", "false")]);
    }
    if let Some(outside) = &c.outside {
        start.extend_attributes([("outside", outside.as_str())]);
    }
    writer.write_event(Event::Empty(start))?;
    Ok(())
}

fn write_species(writer: &mut Writer<Vec<u8>>, s: &Species, _level: u32) -> Result<(), SbmlError> {
    let mut start = BytesStart::new("species");
    start.extend_attributes([("id", s.id.as_str()), ("compartment", s.compartment.as_str())]);
    if let Some(name) = &s.name {
        start.extend_attributes([("name", name.as_str())]);
    }
    if let Some(initial_concentration) = s.initial_concentration {
        start.extend_attributes([(
            "initialConcentration",
            initial_concentration.to_string().as_str(),
        )]);
    }
    if let Some(initial_amount) = s.initial_amount {
        start.extend_attributes([("initialAmount", initial_amount.to_string().as_str())]);
    }
    if let Some(substance_units) = &s.substance_units {
        start.extend_attributes([("substanceUnits", substance_units.as_str())]);
    }
    if s.has_only_substance_units {
        start.extend_attributes([("hasOnlySubstanceUnits", "true")]);
    }
    if !s.boundary_condition {
        start.extend_attributes([("boundaryCondition", "false")]);
    }
    if s.constant {
        start.extend_attributes([("constant", "true")]);
    }
    if let Some(charge) = s.charge {
        start.extend_attributes([("charge", charge.to_string().as_str())]);
    }
    if let Some(fbc_charge) = s.fbc_charge {
        start.extend_attributes([("fbc:charge", fbc_charge.to_string().as_str())]);
    }
    if let Some(chemical_formula) = &s.chemical_formula {
        start.extend_attributes([("fbc:chemicalFormula", chemical_formula.as_str())]);
    }
    writer.write_event(Event::Empty(start))?;
    Ok(())
}

fn write_parameter(writer: &mut Writer<Vec<u8>>, p: &Parameter) -> Result<(), SbmlError> {
    let mut start = BytesStart::new("parameter");
    start.extend_attributes([("id", p.id.as_str())]);
    if let Some(name) = &p.name {
        start.extend_attributes([("name", name.as_str())]);
    }
    if let Some(value) = p.value {
        start.extend_attributes([("value", value.to_string().as_str())]);
    }
    if let Some(units) = &p.units {
        start.extend_attributes([("units", units.as_str())]);
    }
    if !p.constant {
        start.extend_attributes([("constant", "false")]);
    }
    writer.write_event(Event::Empty(start))?;
    Ok(())
}

fn write_initial_assignment(
    writer: &mut Writer<Vec<u8>>,
    ia: &InitialAssignment,
) -> Result<(), SbmlError> {
    writer.write_event(Event::Start(BytesStart::new("initialAssignment")))?;
    writer.write_event(Event::Start(BytesStart::new("symbol")))?;
    writer.write_event(Event::Text(BytesText::new(ia.symbol.as_str())))?;
    writer.write_event(Event::End(BytesEnd::new("symbol")))?;
    write_math(writer, &ia.math)?;
    writer.write_event(Event::End(BytesEnd::new("initialAssignment")))?;
    Ok(())
}

fn write_rule(writer: &mut Writer<Vec<u8>>, r: &Rule) -> Result<(), SbmlError> {
    match r {
        Rule::Assignment { variable, math } => {
            let mut start = BytesStart::new("assignmentRule");
            start.extend_attributes([("variable", variable.as_str())]);
            writer.write_event(Event::Start(start))?;
            write_math(writer, math)?;
            writer.write_event(Event::End(BytesEnd::new("assignmentRule")))?;
        }
        Rule::Rate { variable, math } => {
            let mut start = BytesStart::new("rateRule");
            start.extend_attributes([("variable", variable.as_str())]);
            writer.write_event(Event::Start(start))?;
            write_math(writer, math)?;
            writer.write_event(Event::End(BytesEnd::new("rateRule")))?;
        }
        Rule::Algebraic { variable, math } => {
            let mut start = BytesStart::new("algebraicRule");
            if let Some(variable) = variable {
                start.extend_attributes([("variable", variable.as_str())]);
            }
            writer.write_event(Event::Start(start))?;
            write_math(writer, math)?;
            writer.write_event(Event::End(BytesEnd::new("algebraicRule")))?;
        }
    }
    Ok(())
}

fn write_constraint(writer: &mut Writer<Vec<u8>>, c: &Constraint) -> Result<(), SbmlError> {
    writer.write_event(Event::Start(BytesStart::new("constraint")))?;
    write_math(writer, &c.math)?;
    if let Some(message) = &c.message {
        writer.write_event(Event::Start(BytesStart::new("message")))?;
        writer.write_event(Event::Text(BytesText::new(message.as_str())))?;
        writer.write_event(Event::End(BytesEnd::new("message")))?;
    }
    writer.write_event(Event::End(BytesEnd::new("constraint")))?;
    Ok(())
}

fn write_reaction(
    writer: &mut Writer<Vec<u8>>,
    r: &Reaction,
    _level: u32,
) -> Result<(), SbmlError> {
    let mut start = BytesStart::new("reaction");
    start.extend_attributes([("id", r.id.as_str())]);
    if let Some(name) = &r.name {
        start.extend_attributes([("name", name.as_str())]);
    }
    if !r.reversible {
        start.extend_attributes([("reversible", "false")]);
    }
    if r.fast {
        start.extend_attributes([("fast", "true")]);
    }
    if let Some(lower) = &r.lower_flux_bound {
        start.extend_attributes([("fbc:lowerFluxBound", lower.as_str())]);
    }
    if let Some(upper) = &r.upper_flux_bound {
        start.extend_attributes([("fbc:upperFluxBound", upper.as_str())]);
    }
    if let Some(gpa) = &r.gene_product_association {
        start.extend_attributes([("fbc:geneProductAssociation", gpa.as_str())]);
    }
    writer.write_event(Event::Start(start))?;

    if !r.reactants.is_empty() {
        writer.write_event(Event::Start(BytesStart::new("listOfReactants")))?;
        for sr in &r.reactants {
            write_species_reference(writer, sr, "speciesReference")?;
        }
        writer.write_event(Event::End(BytesEnd::new("listOfReactants")))?;
    }

    if !r.products.is_empty() {
        writer.write_event(Event::Start(BytesStart::new("listOfProducts")))?;
        for sr in &r.products {
            write_species_reference(writer, sr, "speciesReference")?;
        }
        writer.write_event(Event::End(BytesEnd::new("listOfProducts")))?;
    }

    if !r.modifiers.is_empty() {
        writer.write_event(Event::Start(BytesStart::new("listOfModifiers")))?;
        for m in &r.modifiers {
            write_modifier_species_reference(writer, m)?;
        }
        writer.write_event(Event::End(BytesEnd::new("listOfModifiers")))?;
    }

    if let Some(kl) = &r.kinetic_law {
        write_kinetic_law(writer, kl)?;
    }

    writer.write_event(Event::End(BytesEnd::new("reaction")))?;
    Ok(())
}

fn write_species_reference(
    writer: &mut Writer<Vec<u8>>,
    sr: &SpeciesReference,
    tag: &str,
) -> Result<(), SbmlError> {
    let mut start = BytesStart::new(tag);
    start.extend_attributes([("species", sr.species.as_str())]);
    if sr.stoichiometry != 1.0 {
        start.extend_attributes([("stoichiometry", sr.stoichiometry.to_string().as_str())]);
    }
    if !sr.constant {
        start.extend_attributes([("constant", "false")]);
    }
    writer.write_event(Event::Empty(start))?;
    Ok(())
}

fn write_modifier_species_reference(
    writer: &mut Writer<Vec<u8>>,
    m: &ModifierSpeciesReference,
) -> Result<(), SbmlError> {
    let mut start = BytesStart::new("modifierSpeciesReference");
    start.extend_attributes([("species", m.species.as_str())]);
    writer.write_event(Event::Empty(start))?;
    Ok(())
}

fn write_kinetic_law(writer: &mut Writer<Vec<u8>>, kl: &KineticLaw) -> Result<(), SbmlError> {
    writer.write_event(Event::Start(BytesStart::new("kineticLaw")))?;
    if !kl.parameters.is_empty() {
        writer.write_event(Event::Start(BytesStart::new("listOfParameters")))?;
        for p in &kl.parameters {
            write_parameter(writer, p)?;
        }
        writer.write_event(Event::End(BytesEnd::new("listOfParameters")))?;
    }
    if let Some(math) = &kl.math {
        writer.write_event(Event::Start(BytesStart::new("math")))?;
        write_math_node(writer, math)?;
        writer.write_event(Event::End(BytesEnd::new("math")))?;
    }
    writer.write_event(Event::End(BytesEnd::new("kineticLaw")))?;
    Ok(())
}

fn write_event(
    writer: &mut Writer<Vec<u8>>,
    e: &crate::model::Event,
    level: u32,
) -> Result<(), SbmlError> {
    let mut start = BytesStart::new("event");
    if let Some(id) = &e.id {
        start.extend_attributes([("id", id.as_str())]);
    }
    if let Some(name) = &e.name {
        start.extend_attributes([("name", name.as_str())]);
    }
    writer.write_event(Event::Start(start))?;

    if let Some(trigger) = &e.trigger {
        writer.write_event(Event::Start(BytesStart::new("trigger")))?;
        write_math(writer, &trigger.math)?;
        if let Some(initial_value) = trigger.initial_value {
            writer.write_event(Event::Start(BytesStart::new("initialValue")))?;
            writer.write_event(Event::Text(BytesText::new(initial_value.to_string().as_str())))?;
            writer.write_event(Event::End(BytesEnd::new("initialValue")))?;
        }
        if let Some(persistent) = trigger.persistent {
            writer.write_event(Event::Start(BytesStart::new("persistent")))?;
            writer.write_event(Event::Text(BytesText::new(persistent.to_string().as_str())))?;
            writer.write_event(Event::End(BytesEnd::new("persistent")))?;
        }
        writer.write_event(Event::End(BytesEnd::new("trigger")))?;
    }

    if let Some(delay) = &e.delay {
        writer.write_event(Event::Start(BytesStart::new("delay")))?;
        write_math(writer, delay)?;
        writer.write_event(Event::End(BytesEnd::new("delay")))?;
    }

    if level == 3 {
        if let Some(priority) = &e.priority {
            writer.write_event(Event::Start(BytesStart::new("priority")))?;
            write_math(writer, priority)?;
            writer.write_event(Event::End(BytesEnd::new("priority")))?;
        }
    }

    if !e.event_assignments.is_empty() {
        writer.write_event(Event::Start(BytesStart::new("listOfEventAssignments")))?;
        for ea in &e.event_assignments {
            write_event_assignment(writer, ea)?;
        }
        writer.write_event(Event::End(BytesEnd::new("listOfEventAssignments")))?;
    }

    writer.write_event(Event::End(BytesEnd::new("event")))?;
    Ok(())
}

fn write_event_assignment(
    writer: &mut Writer<Vec<u8>>,
    ea: &EventAssignment,
) -> Result<(), SbmlError> {
    let mut start = BytesStart::new("eventAssignment");
    start.extend_attributes([("variable", ea.variable.as_str())]);
    writer.write_event(Event::Start(start))?;
    write_math(writer, &ea.math)?;
    writer.write_event(Event::End(BytesEnd::new("eventAssignment")))?;
    Ok(())
}

fn write_function_definition(
    writer: &mut Writer<Vec<u8>>,
    fd: &FunctionDefinition,
) -> Result<(), SbmlError> {
    let mut start = BytesStart::new("functionDefinition");
    start.extend_attributes([("id", fd.id.as_str())]);
    if let Some(name) = &fd.name {
        start.extend_attributes([("name", name.as_str())]);
    }
    writer.write_event(Event::Start(start))?;
    writer.write_event(Event::Start(BytesStart::new("math")))?;
    writer.write_event(Event::Start(BytesStart::new("lambda")))?;
    if let MathNode::Lambda { ref bvars, ref body } = fd.math {
        for bvar in bvars {
            writer.write_event(Event::Start(BytesStart::new("bvar")))?;
            writer.write_event(Event::Start(BytesStart::new("ci")))?;
            writer.write_event(Event::Text(BytesText::new(bvar.as_str())))?;
            writer.write_event(Event::End(BytesEnd::new("ci")))?;
            writer.write_event(Event::End(BytesEnd::new("bvar")))?;
        }
        write_math_node(writer, body)?;
    }
    writer.write_event(Event::End(BytesEnd::new("lambda")))?;
    writer.write_event(Event::End(BytesEnd::new("math")))?;
    writer.write_event(Event::End(BytesEnd::new("functionDefinition")))?;
    Ok(())
}

fn write_fbc_model(writer: &mut Writer<Vec<u8>>, fbc: &FbcModel) -> Result<(), SbmlError> {
    if !fbc.objectives.is_empty() {
        writer.write_event(Event::Start(BytesStart::new("fbc:listOfObjectives")))?;
        for obj in &fbc.objectives {
            write_fbc_objective(writer, obj)?;
        }
        writer.write_event(Event::End(BytesEnd::new("fbc:listOfObjectives")))?;
    }

    if !fbc.flux_bounds.is_empty() {
        writer.write_event(Event::Start(BytesStart::new("fbc:listOfFluxBounds")))?;
        for fb in &fbc.flux_bounds {
            write_fbc_flux_bound(writer, fb)?;
        }
        writer.write_event(Event::End(BytesEnd::new("fbc:listOfFluxBounds")))?;
    }

    if !fbc.gene_products.is_empty() {
        writer.write_event(Event::Start(BytesStart::new("fbc:listOfGeneProducts")))?;
        for gp in &fbc.gene_products {
            write_fbc_gene_product(writer, gp)?;
        }
        writer.write_event(Event::End(BytesEnd::new("fbc:listOfGeneProducts")))?;
    }

    Ok(())
}

fn write_fbc_objective(writer: &mut Writer<Vec<u8>>, obj: &Objective) -> Result<(), SbmlError> {
    let mut start = BytesStart::new("fbc:objective");
    start.extend_attributes([("fbc:id", obj.id.as_str()), ("fbc:type", obj.type_.as_str())]);
    if let Some(name) = &obj.name {
        start.extend_attributes([("name", name.as_str())]);
    }
    writer.write_event(Event::Start(start))?;
    if !obj.flux_objectives.is_empty() {
        writer.write_event(Event::Start(BytesStart::new("fbc:listOfFluxObjectives")))?;
        for fo in &obj.flux_objectives {
            let mut fo_start = BytesStart::new("fbc:fluxObjective");
            fo_start.extend_attributes([
                ("fbc:reaction", fo.reaction.as_str()),
                ("fbc:coefficient", fo.coefficient.to_string().as_str()),
            ]);
            writer.write_event(Event::Empty(fo_start))?;
        }
        writer.write_event(Event::End(BytesEnd::new("fbc:listOfFluxObjectives")))?;
    }
    writer.write_event(Event::End(BytesEnd::new("fbc:objective")))?;
    Ok(())
}

fn write_fbc_flux_bound(writer: &mut Writer<Vec<u8>>, fb: &FluxBound) -> Result<(), SbmlError> {
    let mut start = BytesStart::new("fbc:fluxBound");
    if let Some(id) = &fb.id {
        start.extend_attributes([("fbc:id", id.as_str())]);
    }
    start.extend_attributes([
        ("fbc:reaction", fb.reaction.as_str()),
        ("fbc:operation", fb.operation.as_str()),
        ("fbc:value", fb.value.to_string().as_str()),
    ]);
    writer.write_event(Event::Empty(start))?;
    Ok(())
}

fn write_fbc_gene_product(writer: &mut Writer<Vec<u8>>, gp: &GeneProduct) -> Result<(), SbmlError> {
    let mut start = BytesStart::new("fbc:geneProduct");
    start.extend_attributes([("fbc:id", gp.id.as_str())]);
    if let Some(name) = &gp.name {
        start.extend_attributes([("name", name.as_str())]);
    }
    if let Some(label) = &gp.label {
        start.extend_attributes([("fbc:label", label.as_str())]);
    }
    if let Some(associated_species) = &gp.associated_species {
        start.extend_attributes([("fbc:associatedSpecies", associated_species.as_str())]);
    }
    writer.write_event(Event::Empty(start))?;
    Ok(())
}

fn write_qual_model(writer: &mut Writer<Vec<u8>>, qual: &QualModel) -> Result<(), SbmlError> {
    if !qual.qualitative_species.is_empty() {
        writer.write_event(Event::Start(BytesStart::new("qual:listOfQualitativeSpecies")))?;
        for qs in &qual.qualitative_species {
            write_qual_species(writer, qs)?;
        }
        writer.write_event(Event::End(BytesEnd::new("qual:listOfQualitativeSpecies")))?;
    }

    if !qual.transitions.is_empty() {
        writer.write_event(Event::Start(BytesStart::new("qual:listOfTransitions")))?;
        for t in &qual.transitions {
            write_qual_transition(writer, t)?;
        }
        writer.write_event(Event::End(BytesEnd::new("qual:listOfTransitions")))?;
    }

    Ok(())
}

fn write_qual_species(writer: &mut Writer<Vec<u8>>, qs: &QualSpecies) -> Result<(), SbmlError> {
    let mut start = BytesStart::new("qual:qualitativeSpecies");
    start.extend_attributes([("qual:id", qs.id.as_str())]);
    if let Some(name) = &qs.name {
        start.extend_attributes([("name", name.as_str())]);
    }
    if let Some(compartment) = &qs.compartment {
        start.extend_attributes([("qual:compartment", compartment.as_str())]);
    }
    if qs.constant {
        start.extend_attributes([("qual:constant", "true")]);
    }
    if let Some(initial_level) = qs.initial_level {
        start.extend_attributes([("qual:initialLevel", initial_level.to_string().as_str())]);
    }
    writer.write_event(Event::Empty(start))?;
    Ok(())
}

fn write_qual_transition(
    writer: &mut Writer<Vec<u8>>,
    t: &QualTransition,
) -> Result<(), SbmlError> {
    let mut start = BytesStart::new("qual:transition");
    if let Some(id) = &t.id {
        start.extend_attributes([("qual:id", id.as_str())]);
    }
    if let Some(name) = &t.name {
        start.extend_attributes([("name", name.as_str())]);
    }
    writer.write_event(Event::Start(start))?;

    if !t.inputs.is_empty() {
        writer.write_event(Event::Start(BytesStart::new("qual:listOfInputs")))?;
        for i in &t.inputs {
            let mut input_start = BytesStart::new("qual:input");
            input_start
                .extend_attributes([("qual:qualitativeSpecies", i.qualitative_species.as_str())]);
            if let Some(sign) = &i.sign {
                input_start.extend_attributes([("qual:sign", sign.as_str())]);
            }
            if let Some(transition_effect) = &i.transition_effect {
                input_start
                    .extend_attributes([("qual:transitionEffect", transition_effect.as_str())]);
            }
            writer.write_event(Event::Empty(input_start))?;
        }
        writer.write_event(Event::End(BytesEnd::new("qual:listOfInputs")))?;
    }

    if !t.outputs.is_empty() {
        writer.write_event(Event::Start(BytesStart::new("qual:listOfOutputs")))?;
        for o in &t.outputs {
            let mut output_start = BytesStart::new("qual:output");
            output_start
                .extend_attributes([("qual:qualitativeSpecies", o.qualitative_species.as_str())]);
            if let Some(transition_effect) = &o.transition_effect {
                output_start
                    .extend_attributes([("qual:transitionEffect", transition_effect.as_str())]);
            }
            if let Some(output_level) = o.output_level {
                output_start
                    .extend_attributes([("qual:outputLevel", output_level.to_string().as_str())]);
            }
            writer.write_event(Event::Empty(output_start))?;
        }
        writer.write_event(Event::End(BytesEnd::new("qual:listOfOutputs")))?;
    }

    if let Some(function) = &t.function {
        writer.write_event(Event::Start(BytesStart::new("qual:defaultTerm")))?;
        write_math(writer, function)?;
        writer.write_event(Event::End(BytesEnd::new("qual:defaultTerm")))?;
    }

    writer.write_event(Event::End(BytesEnd::new("qual:transition")))?;
    Ok(())
}

fn write_spatial_model(
    writer: &mut Writer<Vec<u8>>,
    spatial: &SpatialModel,
) -> Result<(), SbmlError> {
    if let Some(geometry) = &spatial.geometry {
        let mut start = BytesStart::new("spatial:geometry");
        start.extend_attributes([("spatial:id", geometry.id.as_str())]);
        if let Some(dimensions) = geometry.dimensions {
            start.extend_attributes([("spatial:dimensions", dimensions.to_string().as_str())]);
        }
        writer.write_event(Event::Start(start))?;

        if !geometry.coordinate_systems.is_empty() {
            writer.write_event(Event::Start(BytesStart::new("spatial:listOfCoordinateSystems")))?;
            for cs in &geometry.coordinate_systems {
                let mut cs_start = BytesStart::new("spatial:coordinateSystem");
                cs_start.extend_attributes([("spatial:id", cs.as_str())]);
                writer.write_event(Event::Empty(cs_start))?;
            }
            writer.write_event(Event::End(BytesEnd::new("spatial:listOfCoordinateSystems")))?;
        }

        if !geometry.domain_types.is_empty() {
            writer.write_event(Event::Start(BytesStart::new("spatial:listOfDomainTypes")))?;
            for dt in &geometry.domain_types {
                let mut dt_start = BytesStart::new("spatial:domainType");
                dt_start.extend_attributes([("spatial:id", dt.as_str())]);
                writer.write_event(Event::Empty(dt_start))?;
            }
            writer.write_event(Event::End(BytesEnd::new("spatial:listOfDomainTypes")))?;
        }

        if !geometry.domains.is_empty() {
            writer.write_event(Event::Start(BytesStart::new("spatial:listOfDomains")))?;
            for d in &geometry.domains {
                let mut d_start = BytesStart::new("spatial:domain");
                d_start.extend_attributes([("spatial:id", d.as_str())]);
                writer.write_event(Event::Empty(d_start))?;
            }
            writer.write_event(Event::End(BytesEnd::new("spatial:listOfDomains")))?;
        }

        if !geometry.geometry_definitions.is_empty() {
            writer
                .write_event(Event::Start(BytesStart::new("spatial:listOfGeometryDefinitions")))?;
            for gd in &geometry.geometry_definitions {
                let mut gd_start = BytesStart::new("spatial:geometryDefinition");
                gd_start.extend_attributes([("spatial:id", gd.as_str())]);
                writer.write_event(Event::Empty(gd_start))?;
            }
            writer.write_event(Event::End(BytesEnd::new("spatial:listOfGeometryDefinitions")))?;
        }

        writer.write_event(Event::End(BytesEnd::new("spatial:geometry")))?;
    }
    Ok(())
}

fn write_math(writer: &mut Writer<Vec<u8>>, math: &MathNode) -> Result<(), SbmlError> {
    writer.write_event(Event::Start(BytesStart::new("math")))?;
    write_math_node(writer, math)?;
    writer.write_event(Event::End(BytesEnd::new("math")))?;
    Ok(())
}

fn write_math_node(writer: &mut Writer<Vec<u8>>, node: &MathNode) -> Result<(), SbmlError> {
    match node {
        MathNode::Ci(name) => {
            writer.write_event(Event::Start(BytesStart::new("ci")))?;
            writer.write_event(Event::Text(BytesText::new(name.as_str())))?;
            writer.write_event(Event::End(BytesEnd::new("ci")))?;
        }
        MathNode::Cn { type_, kind, units, value } => {
            let mut start = BytesStart::new("cn");
            if let Some(type_) = type_ {
                start.extend_attributes([("type", type_.as_str())]);
            }
            if let Some(kind) = kind {
                start.extend_attributes([("kind", kind.as_str())]);
            }
            if let Some(units) = units {
                start.extend_attributes([("units", units.as_str())]);
            }
            writer.write_event(Event::Start(start))?;
            writer.write_event(Event::Text(BytesText::new(value.as_str())))?;
            writer.write_event(Event::End(BytesEnd::new("cn")))?;
        }
        MathNode::Operator(name) => {
            if name == "true"
                || name == "false"
                || name == "pi"
                || name == "exponentiale"
                || name == "notanumber"
                || name == "infinity"
                || name == "eulergamma"
            {
                writer.write_event(Event::Empty(BytesStart::new(name.as_str())))?;
            } else {
                writer.write_event(Event::Start(BytesStart::new(name.as_str())))?;
                writer.write_event(Event::End(BytesEnd::new(name.as_str())))?;
            }
        }
        MathNode::Apply { op, args } => {
            writer.write_event(Event::Start(BytesStart::new("apply")))?;
            write_math_node(writer, op)?;
            for a in args {
                write_math_node(writer, a)?;
            }
            writer.write_event(Event::End(BytesEnd::new("apply")))?;
        }
        MathNode::Lambda { bvars, body } => {
            writer.write_event(Event::Start(BytesStart::new("lambda")))?;
            for bvar in bvars {
                writer.write_event(Event::Start(BytesStart::new("bvar")))?;
                writer.write_event(Event::Start(BytesStart::new("ci")))?;
                writer.write_event(Event::Text(BytesText::new(bvar.as_str())))?;
                writer.write_event(Event::End(BytesEnd::new("ci")))?;
                writer.write_event(Event::End(BytesEnd::new("bvar")))?;
            }
            write_math_node(writer, body)?;
            writer.write_event(Event::End(BytesEnd::new("lambda")))?;
        }
        MathNode::Piece { value, condition } => {
            writer.write_event(Event::Start(BytesStart::new("piece")))?;
            write_math_node(writer, value)?;
            write_math_node(writer, condition)?;
            writer.write_event(Event::End(BytesEnd::new("piece")))?;
        }
        MathNode::Otherwise(node) => {
            writer.write_event(Event::Start(BytesStart::new("otherwise")))?;
            write_math_node(writer, node)?;
            writer.write_event(Event::End(BytesEnd::new("otherwise")))?;
        }
        MathNode::Piecewise { pieces, otherwise } => {
            writer.write_event(Event::Start(BytesStart::new("piecewise")))?;
            for (v, c) in pieces {
                writer.write_event(Event::Start(BytesStart::new("piece")))?;
                write_math_node(writer, v)?;
                write_math_node(writer, c)?;
                writer.write_event(Event::End(BytesEnd::new("piece")))?;
            }
            if let Some(o) = otherwise {
                writer.write_event(Event::Start(BytesStart::new("otherwise")))?;
                write_math_node(writer, o)?;
                writer.write_event(Event::End(BytesEnd::new("otherwise")))?;
            }
            writer.write_event(Event::End(BytesEnd::new("piecewise")))?;
        }
        MathNode::Csymbol { definition_url, name } => {
            let mut start = BytesStart::new("csymbol");
            if let Some(definition_url) = definition_url {
                start.extend_attributes([("definitionURL", definition_url.as_str())]);
            }
            writer.write_event(Event::Start(start))?;
            writer.write_event(Event::Text(BytesText::new(name.as_str())))?;
            writer.write_event(Event::End(BytesEnd::new("csymbol")))?;
        }
        MathNode::Bvar(_) => {}
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// tpt-io-core Writer trait implementation (requires std)
// ---------------------------------------------------------------------------

#[cfg(feature = "std")]
impl crate::Writer<Sbml> for Sbml {
    type Error = SbmlError;

    fn serialize(&self, _value: &Sbml) -> Result<Vec<u8>, Self::Error> {
        serialize(self)
    }
}

/// Round-trip a parsed [`Sbml`] through the serializer and re-parse it.
#[cfg(feature = "std")]
pub fn round_trip(sbml: &Sbml) -> Result<Sbml, SbmlError> {
    let bytes = serialize(sbml)?;
    Sbml::parse(&bytes)
}
