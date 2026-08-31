//! SBML document parser.
//!
//! Drives a [`quick_xml::NsReader`] over the document, building the typed
//! [`crate::model`] representation. Namespaces are ignored for element/attribute
//! matching (SBML uses fixed element names), so Level 2 and Level 3 documents
//! are handled uniformly.
//!
//! Each recursive parser owns its own scratch buffer: a [`quick_xml`] event
//! borrows the buffer it was read from, so reusing one buffer across a live
//! event would be a borrow error. Child parsers therefore allocate a fresh
//! `Vec<u8>` for their own reads.

use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::str::FromStr;

use quick_xml::events::{BytesEnd, BytesStart, Event};
use quick_xml::NsReader;

use crate::error::SbmlError;
use crate::mathml::{parse_math, skip_to_end};
use crate::model::Event as ModelEvent;
use crate::model::*;

#[cfg(feature = "alloc")]
use tpt_io_core::{Reader, ValidationSettings};

fn ln_str(e: &BytesStart<'_>) -> String {
    String::from_utf8_lossy(e.local_name().as_ref()).into_owned()
}

fn ename(e: &BytesEnd<'_>) -> String {
    String::from_utf8_lossy(e.local_name().as_ref()).into_owned()
}

fn attr_opt(e: &BytesStart, key: &str) -> Option<String> {
    for a in e.attributes().flatten() {
        if core::str::from_utf8(a.key.local_name().as_ref()).ok()? == key {
            return Some(String::from_utf8_lossy(&a.value).into_owned());
        }
    }
    None
}

/// Return `(prefix, value)` for the first attribute whose local name is `key`.
fn attr_detail(e: &BytesStart, key: &str) -> Option<(Option<String>, String)> {
    for a in e.attributes().flatten() {
        if core::str::from_utf8(a.key.local_name().as_ref()).ok()? == key {
            let prefix = a.key.prefix().map(|p| String::from_utf8_lossy(p.as_ref()).into_owned());
            return Some((prefix, String::from_utf8_lossy(&a.value).into_owned()));
        }
    }
    None
}

fn req_attr(e: &BytesStart, key: &str) -> Result<String, SbmlError> {
    attr_opt(e, key).ok_or_else(|| SbmlError::missing_attr(ln_str(e).as_str(), key))
}

fn opt_f64_attr(e: &BytesStart, key: &str) -> Result<Option<f64>, SbmlError> {
    match attr_opt(e, key) {
        None => Ok(None),
        Some(v) => f64::from_str(&v)
            .map(Some)
            .map_err(|_| SbmlError::invalid_value(ln_str(e).as_str(), key, v)),
    }
}

fn opt_i64_attr(e: &BytesStart, key: &str) -> Result<Option<i64>, SbmlError> {
    match attr_opt(e, key) {
        None => Ok(None),
        Some(v) => i64::from_str(&v)
            .map(Some)
            .map_err(|_| SbmlError::invalid_value(ln_str(e).as_str(), key, v)),
    }
}

fn bool_attr(e: &BytesStart, key: &str, default: bool) -> bool {
    match attr_opt(e, key) {
        Some(v) => v == "true",
        None => default,
    }
}

// ---------------------------------------------------------------------------
// Top-level driver
// ---------------------------------------------------------------------------

/// Parse a complete SBML document from bytes.
pub fn parse_sbml(input: &[u8]) -> Result<Sbml, SbmlError> {
    let mut reader = NsReader::from_str(
        core::str::from_utf8(input)
            .map_err(|_| SbmlError::Core(tpt_io_core::ParseError::invalid_byte(0, 0)))?,
    );
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf)? {
            Event::Start(e) if ln_str(&e) == "sbml" => {
                let level = opt_i64_attr(&e, "level")?.map(|v| v as u32).unwrap_or(0);
                let version = opt_i64_attr(&e, "version")?.map(|v| v as u32).unwrap_or(0);
                let model = parse_sbml_model(&mut reader, &mut buf)?;
                return Ok(Sbml { level, version, model });
            }
            Event::Eof => return Err(SbmlError::Core(tpt_io_core::ParseError::eof(0))),
            _ => {}
        }
    }
}

impl Sbml {
    /// Convenience wrapper around [`parse_sbml`].
    pub fn parse(input: &[u8]) -> Result<Self, SbmlError> {
        parse_sbml(input)
    }
}

/// Read the children of the `<sbml>` element, delegating `<model>` to
/// [`parse_model_element`].
fn parse_sbml_model(reader: &mut NsReader<&[u8]>, buf: &mut Vec<u8>) -> Result<Model, SbmlError> {
    let mut model = Model::default();
    loop {
        match reader.read_event_into(buf)? {
            Event::Start(child) if ln_str(&child) == "model" => {
                model = parse_model_element(reader, &child)?
            }
            Event::End(e) if ename(&e) == "sbml" => return Ok(model),
            Event::Eof => return Err(SbmlError::Core(tpt_io_core::ParseError::eof(0))),
            _ => {}
        }
    }
}

/// Read the `<model>` element (already opened by the caller) and its children.
pub(crate) fn parse_model_element(
    reader: &mut NsReader<&[u8]>,
    e: &BytesStart,
) -> Result<Model, SbmlError> {
    let mut model = Model::default();
    model.id = attr_opt(e, "id");
    model.name = attr_opt(e, "name");
    model.substance_units = attr_opt(e, "substanceUnits");
    model.time_units = attr_opt(e, "timeUnits");
    model.volume_units = attr_opt(e, "volumeUnits");
    model.area_units = attr_opt(e, "areaUnits");
    model.length_units = attr_opt(e, "lengthUnits");
    model.extent_units = attr_opt(e, "extentUnits");
    model.conversion_factor = attr_opt(e, "conversionFactor");
    if let Some(v) = attr_opt(e, "strict") {
        if v == "true" {
            model.fbc.get_or_insert_with(FbcModel::default).strict = true;
        }
    }
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf)? {
            Event::Start(c) => match ln_str(&c).as_str() {
                "listOfUnitDefinitions" => {
                    parse_unit_definition_list(reader, &mut model.unit_definitions)?
                }
                "listOfCompartments" => parse_compartment_list(reader, &mut model.compartments)?,
                "listOfSpecies" => parse_species_list(reader, &mut model.species)?,
                "listOfParameters" => parse_parameter_list(reader, &mut model.parameters)?,
                "listOfInitialAssignments" => {
                    parse_initial_assignment_list(reader, &mut model.initial_assignments)?
                }
                "listOfRules" => parse_rule_list(reader, &mut model.rules)?,
                "listOfConstraints" => parse_constraint_list(reader, &mut model.constraints)?,
                "listOfReactions" => parse_reaction_list(reader, &mut model.reactions)?,
                "listOfEvents" => parse_event_list(reader, &mut model.events)?,
                "listOfFunctionDefinitions" => {
                    parse_function_definition_list(reader, &mut model.function_definitions)?
                }
                "listOfObjectives" => parse_fbc_objectives(reader, &mut model)?,
                "listOfFluxBounds" => parse_fbc_flux_bounds(reader, &mut model)?,
                "listOfGeneProducts" => parse_fbc_gene_products(reader, &mut model)?,
                "listOfQualitativeSpecies" => parse_qual_species_list(reader, &mut model)?,
                "listOfTransitions" => parse_qual_transition_list(reader, &mut model)?,
                "geometry" => parse_spatial_geometry(reader, &c, &mut model)?,
                _ => {}
            },
            Event::End(end) if ename(&end) == "model" => return Ok(model),
            Event::Eof => return Err(SbmlError::MathParse("unexpected EOF inside <model>".into())),
            _ => {}
        }
    }
}

// ---------------------------------------------------------------------------
// List parsers
// ---------------------------------------------------------------------------

fn parse_unit_definition_list(
    reader: &mut NsReader<&[u8]>,
    out: &mut Vec<UnitDefinition>,
) -> Result<(), SbmlError> {
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf)? {
            Event::Start(c) if ln_str(&c) == "unitDefinition" => {
                out.push(parse_unit_definition(reader, &c)?)
            }
            Event::End(e) if ename(&e) == "listOfUnitDefinitions" => return Ok(()),
            Event::Eof => {
                return Err(SbmlError::MathParse("unexpected EOF in listOfUnitDefinitions".into()))
            }
            _ => {}
        }
    }
}

fn parse_unit_definition(
    reader: &mut NsReader<&[u8]>,
    e: &BytesStart,
) -> Result<UnitDefinition, SbmlError> {
    let mut ud = UnitDefinition::default();
    ud.id = req_attr(e, "id")?;
    ud.name = attr_opt(e, "name");
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf)? {
            Event::Start(c) if ln_str(&c) == "listOfUnits" => {
                let mut inner = Vec::new();
                loop {
                    match reader.read_event_into(&mut inner)? {
                        Event::Start(u) if ln_str(&u) == "unit" => {
                            ud.units.push(parse_unit(&u)?);
                            skip_to_end(reader, "unit")?;
                        }
                        Event::Empty(u) if ln_str(&u) == "unit" => {
                            ud.units.push(parse_unit(&u)?);
                        }
                        Event::End(end) if ename(&end) == "listOfUnits" => break,
                        Event::Eof => {
                            return Err(SbmlError::MathParse(
                                "unexpected EOF in listOfUnits".into(),
                            ))
                        }
                        _ => {}
                    }
                }
            }
            Event::End(end) if ename(&end) == "unitDefinition" => return Ok(ud),
            Event::Eof => {
                return Err(SbmlError::MathParse("unexpected EOF in unitDefinition".into()))
            }
            _ => {}
        }
    }
}

fn parse_unit(e: &BytesStart) -> Result<Unit, SbmlError> {
    let mut u = Unit::default();
    u.kind = req_attr(e, "kind")?;
    u.exponent = opt_f64_attr(e, "exponent")?.unwrap_or(1.0);
    u.scale = opt_i64_attr(e, "scale")?.unwrap_or(0);
    u.multiplier = opt_f64_attr(e, "multiplier")?.unwrap_or(1.0);
    Ok(u)
}

fn parse_compartment_list(
    reader: &mut NsReader<&[u8]>,
    out: &mut Vec<Compartment>,
) -> Result<(), SbmlError> {
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf)? {
            Event::Start(c) if ln_str(&c) == "compartment" => {
                out.push(parse_compartment(reader, &c)?);
                skip_to_end(reader, "compartment")?;
            }
            Event::Empty(c) if ln_str(&c) == "compartment" => {
                out.push(parse_compartment(reader, &c)?)
            }
            Event::End(e) if ename(&e) == "listOfCompartments" => return Ok(()),
            Event::Eof => {
                return Err(SbmlError::MathParse("unexpected EOF in listOfCompartments".into()))
            }
            _ => {}
        }
    }
}

fn parse_compartment(
    _reader: &mut NsReader<&[u8]>,
    e: &BytesStart,
) -> Result<Compartment, SbmlError> {
    let mut c = Compartment::default();
    c.id = req_attr(e, "id")?;
    c.name = attr_opt(e, "name");
    c.spatial_dimensions = opt_f64_attr(e, "spatialDimensions")?;
    c.size = opt_f64_attr(e, "size")?;
    c.units = attr_opt(e, "units");
    c.constant = bool_attr(e, "constant", true);
    c.outside = attr_opt(e, "outside");
    Ok(c)
}

fn parse_species_list(
    reader: &mut NsReader<&[u8]>,
    out: &mut Vec<Species>,
) -> Result<(), SbmlError> {
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf)? {
            Event::Start(c) if ln_str(&c) == "species" => {
                out.push(parse_species(reader, &c)?);
                skip_to_end(reader, "species")?;
            }
            Event::Empty(c) if ln_str(&c) == "species" => out.push(parse_species(reader, &c)?),
            Event::End(e) if ename(&e) == "listOfSpecies" => return Ok(()),
            Event::Eof => {
                return Err(SbmlError::MathParse("unexpected EOF in listOfSpecies".into()))
            }
            _ => {}
        }
    }
}

fn parse_species(_reader: &mut NsReader<&[u8]>, e: &BytesStart) -> Result<Species, SbmlError> {
    let mut s = Species::default();
    s.id = req_attr(e, "id")?;
    s.name = attr_opt(e, "name");
    s.compartment = req_attr(e, "compartment")?;
    s.initial_concentration = opt_f64_attr(e, "initialConcentration")?;
    s.initial_amount = opt_f64_attr(e, "initialAmount")?;
    s.substance_units = attr_opt(e, "substanceUnits");
    s.has_only_substance_units = bool_attr(e, "hasOnlySubstanceUnits", false);
    s.boundary_condition = bool_attr(e, "boundaryCondition", false);
    s.constant = bool_attr(e, "constant", false);
    if let Some((prefix, value)) = attr_detail(e, "charge") {
        let parsed = value.parse::<i64>().ok();
        if prefix.as_deref() == Some("fbc") {
            s.fbc_charge = parsed;
        } else {
            s.charge = parsed;
        }
    }
    s.chemical_formula = attr_opt(e, "chemicalFormula");
    Ok(s)
}

fn parse_parameter_list(
    reader: &mut NsReader<&[u8]>,
    out: &mut Vec<Parameter>,
) -> Result<(), SbmlError> {
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf)? {
            Event::Start(c) if ln_str(&c) == "parameter" => {
                out.push(parse_parameter(reader, &c)?);
                skip_to_end(reader, "parameter")?;
            }
            Event::Empty(c) if ln_str(&c) == "parameter" => out.push(parse_parameter(reader, &c)?),
            Event::End(e) if ename(&e) == "listOfParameters" => return Ok(()),
            Event::Eof => {
                return Err(SbmlError::MathParse("unexpected EOF in listOfParameters".into()))
            }
            _ => {}
        }
    }
}

fn parse_parameter(_reader: &mut NsReader<&[u8]>, e: &BytesStart) -> Result<Parameter, SbmlError> {
    let mut p = Parameter::default();
    p.id = req_attr(e, "id")?;
    p.name = attr_opt(e, "name");
    p.value = opt_f64_attr(e, "value")?;
    p.units = attr_opt(e, "units");
    p.constant = bool_attr(e, "constant", true);
    Ok(p)
}

fn parse_initial_assignment_list(
    reader: &mut NsReader<&[u8]>,
    out: &mut Vec<InitialAssignment>,
) -> Result<(), SbmlError> {
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf)? {
            Event::Start(c) if ln_str(&c) == "initialAssignment" => {
                out.push(parse_initial_assignment(reader, &c)?)
            }
            Event::End(e) if ename(&e) == "listOfInitialAssignments" => return Ok(()),
            Event::Eof => {
                return Err(SbmlError::MathParse(
                    "unexpected EOF in listOfInitialAssignments".into(),
                ))
            }
            _ => {}
        }
    }
}

fn parse_initial_assignment(
    reader: &mut NsReader<&[u8]>,
    e: &BytesStart,
) -> Result<InitialAssignment, SbmlError> {
    let symbol = req_attr(e, "symbol")?;
    let mut math = None;
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf)? {
            Event::Start(c) if ln_str(&c) == "math" => math = Some(parse_math(reader)?),
            Event::End(end) if ename(&end) == "initialAssignment" => {
                return Ok(InitialAssignment {
                    symbol,
                    math: math.ok_or(SbmlError::MissingElement {
                        parent: "initialAssignment",
                        child: "math",
                    })?,
                })
            }
            Event::Eof => {
                return Err(SbmlError::MathParse("unexpected EOF in initialAssignment".into()))
            }
            _ => {}
        }
    }
}

fn parse_rule_list(reader: &mut NsReader<&[u8]>, out: &mut Vec<Rule>) -> Result<(), SbmlError> {
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf)? {
            Event::Start(c) => match ln_str(&c).as_str() {
                "assignmentRule" => out.push(parse_assignment_rule(reader, &c)?),
                "rateRule" => out.push(parse_rate_rule(reader, &c)?),
                "algebraicRule" => out.push(parse_algebraic_rule(reader, &c)?),
                _ => {}
            },
            Event::End(e) if ename(&e) == "listOfRules" => return Ok(()),
            Event::Eof => return Err(SbmlError::MathParse("unexpected EOF in listOfRules".into())),
            _ => {}
        }
    }
}

fn parse_assignment_rule(reader: &mut NsReader<&[u8]>, e: &BytesStart) -> Result<Rule, SbmlError> {
    let variable = req_attr(e, "variable")?;
    let mut math = None;
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf)? {
            Event::Start(c) if ln_str(&c) == "math" => math = Some(parse_math(reader)?),
            Event::End(end) if ename(&end) == "assignmentRule" => {
                return Ok(Rule::Assignment {
                    variable,
                    math: math.ok_or(SbmlError::MissingElement {
                        parent: "assignmentRule",
                        child: "math",
                    })?,
                })
            }
            Event::Eof => {
                return Err(SbmlError::MathParse("unexpected EOF in assignmentRule".into()))
            }
            _ => {}
        }
    }
}

fn parse_rate_rule(reader: &mut NsReader<&[u8]>, e: &BytesStart) -> Result<Rule, SbmlError> {
    let variable = req_attr(e, "variable")?;
    let mut math = None;
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf)? {
            Event::Start(c) if ln_str(&c) == "math" => math = Some(parse_math(reader)?),
            Event::End(end) if ename(&end) == "rateRule" => {
                return Ok(Rule::Rate {
                    variable,
                    math: math
                        .ok_or(SbmlError::MissingElement { parent: "rateRule", child: "math" })?,
                })
            }
            Event::Eof => return Err(SbmlError::MathParse("unexpected EOF in rateRule".into())),
            _ => {}
        }
    }
}

fn parse_algebraic_rule(reader: &mut NsReader<&[u8]>, e: &BytesStart) -> Result<Rule, SbmlError> {
    let variable = attr_opt(e, "variable");
    let mut math = None;
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf)? {
            Event::Start(c) if ln_str(&c) == "math" => math = Some(parse_math(reader)?),
            Event::End(end) if ename(&end) == "algebraicRule" => {
                return Ok(Rule::Algebraic {
                    variable,
                    math: math.ok_or(SbmlError::MissingElement {
                        parent: "algebraicRule",
                        child: "math",
                    })?,
                })
            }
            Event::Eof => {
                return Err(SbmlError::MathParse("unexpected EOF in algebraicRule".into()))
            }
            _ => {}
        }
    }
}

fn parse_constraint_list(
    reader: &mut NsReader<&[u8]>,
    out: &mut Vec<Constraint>,
) -> Result<(), SbmlError> {
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf)? {
            Event::Start(c) if ln_str(&c) == "constraint" => {
                out.push(parse_constraint(reader, &c)?)
            }
            Event::End(e) if ename(&e) == "listOfConstraints" => return Ok(()),
            Event::Eof => {
                return Err(SbmlError::MathParse("unexpected EOF in listOfConstraints".into()))
            }
            _ => {}
        }
    }
}

fn parse_constraint(reader: &mut NsReader<&[u8]>, e: &BytesStart) -> Result<Constraint, SbmlError> {
    let _ = e;
    let mut math = None;
    let mut message = None;
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf)? {
            Event::Start(c) if ln_str(&c) == "math" => math = Some(parse_math(reader)?),
            Event::Start(c) if ln_str(&c) == "message" => {
                message = Some(read_inner_text(reader, "message")?)
            }
            Event::End(end) if ename(&end) == "constraint" => {
                return Ok(Constraint {
                    math: math
                        .ok_or(SbmlError::MissingElement { parent: "constraint", child: "math" })?,
                    message,
                })
            }
            Event::Eof => return Err(SbmlError::MathParse("unexpected EOF in constraint".into())),
            _ => {}
        }
    }
}

fn parse_reaction_list(
    reader: &mut NsReader<&[u8]>,
    out: &mut Vec<Reaction>,
) -> Result<(), SbmlError> {
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf)? {
            Event::Start(c) if ln_str(&c) == "reaction" => out.push(parse_reaction(reader, &c)?),
            Event::End(e) if ename(&e) == "listOfReactions" => return Ok(()),
            Event::Eof => {
                return Err(SbmlError::MathParse("unexpected EOF in listOfReactions".into()))
            }
            _ => {}
        }
    }
}

fn parse_reaction(reader: &mut NsReader<&[u8]>, e: &BytesStart) -> Result<Reaction, SbmlError> {
    let mut r = Reaction::default();
    r.id = req_attr(e, "id")?;
    r.name = attr_opt(e, "name");
    r.reversible = bool_attr(e, "reversible", true);
    r.fast = bool_attr(e, "fast", false);
    r.lower_flux_bound = attr_opt(e, "lowerFluxBound");
    r.upper_flux_bound = attr_opt(e, "upperFluxBound");
    r.gene_product_association = attr_opt(e, "geneProductAssociation");
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf)? {
            Event::Start(c) => match ln_str(&c).as_str() {
                "listOfReactants" => {
                    parse_species_ref_list(reader, "listOfReactants", &mut r.reactants)?
                }
                "listOfProducts" => {
                    parse_species_ref_list(reader, "listOfProducts", &mut r.products)?
                }
                "listOfModifiers" => parse_modifier_list(reader, &mut r.modifiers)?,
                "kineticLaw" => r.kinetic_law = Some(parse_kinetic_law(reader, &c)?),
                _ => {}
            },
            Event::End(end) if ename(&end) == "reaction" => return Ok(r),
            Event::Eof => return Err(SbmlError::MathParse("unexpected EOF in reaction".into())),
            _ => {}
        }
    }
}

fn parse_species_ref_list(
    reader: &mut NsReader<&[u8]>,
    end_name: &str,
    out: &mut Vec<SpeciesReference>,
) -> Result<(), SbmlError> {
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf)? {
            Event::Start(c) if ln_str(&c) == "speciesReference" => {
                out.push(parse_species_ref(reader, &c)?);
                skip_to_end(reader, "speciesReference")?;
            }
            Event::Empty(c) if ln_str(&c) == "speciesReference" => {
                out.push(parse_species_ref(reader, &c)?)
            }
            Event::End(e) if ename(&e) == end_name => return Ok(()),
            Event::Eof => {
                return Err(SbmlError::MathParse("unexpected EOF in speciesRef list".into()))
            }
            _ => {}
        }
    }
}

fn parse_species_ref(
    _reader: &mut NsReader<&[u8]>,
    e: &BytesStart,
) -> Result<SpeciesReference, SbmlError> {
    let mut sr = SpeciesReference::default();
    sr.id = attr_opt(e, "id");
    sr.name = attr_opt(e, "name");
    sr.species = req_attr(e, "species")?;
    sr.stoichiometry = opt_f64_attr(e, "stoichiometry")?.unwrap_or(1.0);
    sr.constant = bool_attr(e, "constant", true);
    Ok(sr)
}

fn parse_modifier_list(
    reader: &mut NsReader<&[u8]>,
    out: &mut Vec<ModifierSpeciesReference>,
) -> Result<(), SbmlError> {
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf)? {
            Event::Start(c) if ln_str(&c) == "modifierSpeciesReference" => {
                let mut m = ModifierSpeciesReference::default();
                m.id = attr_opt(&c, "id");
                m.name = attr_opt(&c, "name");
                m.species = req_attr(&c, "species")?;
                out.push(m);
                skip_to_end(reader, "modifierSpeciesReference")?;
            }
            Event::Empty(c) if ln_str(&c) == "modifierSpeciesReference" => {
                let mut m = ModifierSpeciesReference::default();
                m.id = attr_opt(&c, "id");
                m.name = attr_opt(&c, "name");
                m.species = req_attr(&c, "species")?;
                out.push(m);
            }
            Event::End(e) if ename(&e) == "listOfModifiers" => return Ok(()),
            Event::Eof => {
                return Err(SbmlError::MathParse("unexpected EOF in listOfModifiers".into()))
            }
            _ => {}
        }
    }
}

fn parse_kinetic_law(
    reader: &mut NsReader<&[u8]>,
    e: &BytesStart,
) -> Result<KineticLaw, SbmlError> {
    let mut kl = KineticLaw::default();
    kl.substance_units = attr_opt(e, "substanceUnits");
    kl.time_units = attr_opt(e, "timeUnits");
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf)? {
            Event::Start(c) => match ln_str(&c).as_str() {
                "math" => kl.math = Some(parse_math(reader)?),
                "listOfParameters" => parse_parameter_list(reader, &mut kl.parameters)?,
                _ => {}
            },
            Event::End(end) if ename(&end) == "kineticLaw" => return Ok(kl),
            Event::Eof => return Err(SbmlError::MathParse("unexpected EOF in kineticLaw".into())),
            _ => {}
        }
    }
}

fn parse_event_list(
    reader: &mut NsReader<&[u8]>,
    out: &mut Vec<ModelEvent>,
) -> Result<(), SbmlError> {
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf)? {
            Event::Start(c) if ln_str(&c) == "event" => out.push(parse_event(reader, &c)?),
            Event::End(e) if ename(&e) == "listOfEvents" => return Ok(()),
            Event::Eof => {
                return Err(SbmlError::MathParse("unexpected EOF in listOfEvents".into()))
            }
            _ => {}
        }
    }
}

fn parse_event(reader: &mut NsReader<&[u8]>, e: &BytesStart) -> Result<ModelEvent, SbmlError> {
    let mut ev = ModelEvent::default();
    ev.id = attr_opt(e, "id");
    ev.name = attr_opt(e, "name");
    ev.time_units = attr_opt(e, "timeUnits");
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf)? {
            Event::Start(c) => match ln_str(&c).as_str() {
                "trigger" => ev.trigger = Some(parse_trigger(reader, &c)?),
                "delay" => {
                    let mut dbuf = Vec::new();
                    loop {
                        match reader.read_event_into(&mut dbuf)? {
                            Event::Start(cc) if ln_str(&cc) == "math" => {
                                ev.delay = Some(parse_math(reader)?)
                            }
                            Event::End(end) if ename(&end) == "delay" => break,
                            Event::Eof => {
                                return Err(SbmlError::MathParse("unexpected EOF in delay".into()))
                            }
                            _ => {}
                        }
                    }
                }
                "priority" => {
                    let mut pbuf = Vec::new();
                    loop {
                        match reader.read_event_into(&mut pbuf)? {
                            Event::Start(cc) if ln_str(&cc) == "math" => {
                                ev.priority = Some(parse_math(reader)?)
                            }
                            Event::End(end) if ename(&end) == "priority" => break,
                            Event::Eof => {
                                return Err(SbmlError::MathParse(
                                    "unexpected EOF in priority".into(),
                                ))
                            }
                            _ => {}
                        }
                    }
                }
                "listOfEventAssignments" => {
                    parse_event_assignment_list(reader, &mut ev.event_assignments)?
                }
                _ => {}
            },
            Event::End(end) if ename(&end) == "event" => return Ok(ev),
            Event::Eof => return Err(SbmlError::MathParse("unexpected EOF in event".into())),
            _ => {}
        }
    }
}

fn parse_trigger(reader: &mut NsReader<&[u8]>, e: &BytesStart) -> Result<Trigger, SbmlError> {
    let initial_value = attr_opt(e, "initialValue").map(|v| v == "true");
    let persistent = attr_opt(e, "persistent").map(|v| v == "true");
    let mut math = None;
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf)? {
            Event::Start(c) if ln_str(&c) == "math" => math = Some(parse_math(reader)?),
            Event::End(end) if ename(&end) == "trigger" => {
                return Ok(Trigger {
                    math: math
                        .ok_or(SbmlError::MissingElement { parent: "trigger", child: "math" })?,
                    initial_value,
                    persistent,
                })
            }
            Event::Eof => return Err(SbmlError::MathParse("unexpected EOF in trigger".into())),
            _ => {}
        }
    }
}

fn parse_event_assignment_list(
    reader: &mut NsReader<&[u8]>,
    out: &mut Vec<EventAssignment>,
) -> Result<(), SbmlError> {
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf)? {
            Event::Start(c) if ln_str(&c) == "eventAssignment" => {
                out.push(parse_event_assignment(reader, &c)?)
            }
            Event::End(e) if ename(&e) == "listOfEventAssignments" => return Ok(()),
            Event::Eof => {
                return Err(SbmlError::MathParse("unexpected EOF in listOfEventAssignments".into()))
            }
            _ => {}
        }
    }
}

fn parse_event_assignment(
    reader: &mut NsReader<&[u8]>,
    e: &BytesStart,
) -> Result<EventAssignment, SbmlError> {
    let variable = req_attr(e, "variable")?;
    let mut math = None;
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf)? {
            Event::Start(c) if ln_str(&c) == "math" => math = Some(parse_math(reader)?),
            Event::End(end) if ename(&end) == "eventAssignment" => {
                return Ok(EventAssignment {
                    variable,
                    math: math.ok_or(SbmlError::MissingElement {
                        parent: "eventAssignment",
                        child: "math",
                    })?,
                })
            }
            Event::Eof => {
                return Err(SbmlError::MathParse("unexpected EOF in eventAssignment".into()))
            }
            _ => {}
        }
    }
}

fn parse_function_definition_list(
    reader: &mut NsReader<&[u8]>,
    out: &mut Vec<FunctionDefinition>,
) -> Result<(), SbmlError> {
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf)? {
            Event::Start(c) if ln_str(&c) == "functionDefinition" => {
                out.push(parse_function_definition(reader, &c)?)
            }
            Event::End(e) if ename(&e) == "listOfFunctionDefinitions" => return Ok(()),
            Event::Eof => {
                return Err(SbmlError::MathParse(
                    "unexpected EOF in listOfFunctionDefinitions".into(),
                ))
            }
            _ => {}
        }
    }
}

fn parse_function_definition(
    reader: &mut NsReader<&[u8]>,
    e: &BytesStart,
) -> Result<FunctionDefinition, SbmlError> {
    let id = req_attr(e, "id")?;
    let name = attr_opt(e, "name");
    let mut math = None;
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf)? {
            Event::Start(c) if ln_str(&c) == "math" => math = Some(parse_math(reader)?),
            Event::End(end) if ename(&end) == "functionDefinition" => {
                return Ok(FunctionDefinition {
                    id,
                    name,
                    math: math.ok_or(SbmlError::MissingElement {
                        parent: "functionDefinition",
                        child: "math",
                    })?,
                })
            }
            Event::Eof => {
                return Err(SbmlError::MathParse("unexpected EOF in functionDefinition".into()))
            }
            _ => {}
        }
    }
}

fn read_inner_text(reader: &mut NsReader<&[u8]>, end: &str) -> Result<String, SbmlError> {
    let mut buf = Vec::new();
    let mut text = String::new();
    loop {
        match reader.read_event_into(&mut buf)? {
            Event::Text(t) => text.push_str(&t.unescape()?),
            Event::End(e) if ename(&e) == end => return Ok(text),
            Event::Eof => return Err(SbmlError::MathParse(format!("unexpected EOF in <{end}>"))),
            _ => {}
        }
    }
}

// ---------------------------------------------------------------------------
// Package parsers
// ---------------------------------------------------------------------------

fn parse_fbc_objectives(reader: &mut NsReader<&[u8]>, model: &mut Model) -> Result<(), SbmlError> {
    let fbc = model.fbc.get_or_insert_with(FbcModel::default);
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf)? {
            Event::Start(c) if ln_str(&c) == "objective" => {
                fbc.objectives.push(parse_fbc_objective(reader, &c)?)
            }
            Event::End(e) if ename(&e) == "listOfObjectives" => return Ok(()),
            Event::Eof => {
                return Err(SbmlError::MathParse("unexpected EOF in listOfObjectives".into()))
            }
            _ => {}
        }
    }
}

fn parse_fbc_objective(
    reader: &mut NsReader<&[u8]>,
    e: &BytesStart,
) -> Result<Objective, SbmlError> {
    let mut o = Objective::default();
    o.id = req_attr(e, "id")?;
    o.name = attr_opt(e, "name");
    o.type_ = attr_opt(e, "type").unwrap_or_else(|| "maximize".to_string());
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf)? {
            Event::Start(c) if ln_str(&c) == "listOfFluxObjectives" => {
                let mut inner = Vec::new();
                loop {
                    match reader.read_event_into(&mut inner)? {
                        Event::Start(u) if ln_str(&u) == "fluxObjective" => {
                            o.flux_objectives.push(parse_fbc_flux_objective(&u)?);
                            skip_to_end(reader, "fluxObjective")?;
                        }
                        Event::Empty(u) if ln_str(&u) == "fluxObjective" => {
                            o.flux_objectives.push(parse_fbc_flux_objective(&u)?)
                        }
                        Event::End(end) if ename(&end) == "listOfFluxObjectives" => break,
                        Event::Eof => return Err(SbmlError::MathParse("unexpected EOF".into())),
                        _ => {}
                    }
                }
            }
            Event::End(end) if ename(&end) == "objective" => return Ok(o),
            Event::Eof => return Err(SbmlError::MathParse("unexpected EOF in objective".into())),
            _ => {}
        }
    }
}

fn parse_fbc_flux_objective(e: &BytesStart) -> Result<FluxObjective, SbmlError> {
    let mut fo = FluxObjective::default();
    fo.reaction = req_attr(e, "reaction")?;
    fo.coefficient = opt_f64_attr(e, "coefficient")?.unwrap_or(1.0);
    Ok(fo)
}

fn parse_fbc_flux_bounds(reader: &mut NsReader<&[u8]>, model: &mut Model) -> Result<(), SbmlError> {
    let fbc = model.fbc.get_or_insert_with(FbcModel::default);
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf)? {
            Event::Start(c) if ln_str(&c) == "fluxBound" => {
                fbc.flux_bounds.push(parse_fbc_flux_bound(&c)?);
                skip_to_end(reader, "fluxBound")?;
            }
            Event::Empty(c) if ln_str(&c) == "fluxBound" => {
                fbc.flux_bounds.push(parse_fbc_flux_bound(&c)?)
            }
            Event::End(e) if ename(&e) == "listOfFluxBounds" => return Ok(()),
            Event::Eof => {
                return Err(SbmlError::MathParse("unexpected EOF in listOfFluxBounds".into()))
            }
            _ => {}
        }
    }
}

fn parse_fbc_flux_bound(e: &BytesStart) -> Result<FluxBound, SbmlError> {
    let mut fb = FluxBound::default();
    fb.id = attr_opt(e, "id");
    fb.reaction = req_attr(e, "reaction")?;
    fb.operation = attr_opt(e, "operation").unwrap_or_else(|| "equal".to_string());
    fb.value = opt_f64_attr(e, "value")?.unwrap_or(0.0);
    Ok(fb)
}

fn parse_fbc_gene_products(
    reader: &mut NsReader<&[u8]>,
    model: &mut Model,
) -> Result<(), SbmlError> {
    let fbc = model.fbc.get_or_insert_with(FbcModel::default);
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf)? {
            Event::Start(c) if ln_str(&c) == "geneProduct" => {
                fbc.gene_products.push(parse_fbc_gene_product(&c)?);
                skip_to_end(reader, "geneProduct")?;
            }
            Event::Empty(c) if ln_str(&c) == "geneProduct" => {
                fbc.gene_products.push(parse_fbc_gene_product(&c)?)
            }
            Event::End(e) if ename(&e) == "listOfGeneProducts" => return Ok(()),
            Event::Eof => {
                return Err(SbmlError::MathParse("unexpected EOF in listOfGeneProducts".into()))
            }
            _ => {}
        }
    }
}

fn parse_fbc_gene_product(e: &BytesStart) -> Result<GeneProduct, SbmlError> {
    let mut g = GeneProduct::default();
    g.id = req_attr(e, "id")?;
    g.name = attr_opt(e, "name");
    g.label = attr_opt(e, "label");
    g.associated_species = attr_opt(e, "associatedSpecies");
    Ok(g)
}

fn parse_qual_species_list(
    reader: &mut NsReader<&[u8]>,
    model: &mut Model,
) -> Result<(), SbmlError> {
    let qual = model.qual.get_or_insert_with(QualModel::default);
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf)? {
            Event::Start(c) if ln_str(&c) == "qualitativeSpecies" => {
                qual.qualitative_species.push(parse_qual_species(&c)?);
                skip_to_end(reader, "qualitativeSpecies")?;
            }
            Event::Empty(c) if ln_str(&c) == "qualitativeSpecies" => {
                qual.qualitative_species.push(parse_qual_species(&c)?)
            }
            Event::End(e) if ename(&e) == "listOfQualitativeSpecies" => return Ok(()),
            Event::Eof => {
                return Err(SbmlError::MathParse(
                    "unexpected EOF in listOfQualitativeSpecies".into(),
                ))
            }
            _ => {}
        }
    }
}

fn parse_qual_species(e: &BytesStart) -> Result<QualSpecies, SbmlError> {
    let mut s = QualSpecies::default();
    s.id = req_attr(e, "id")?;
    s.name = attr_opt(e, "name");
    s.compartment = attr_opt(e, "compartment");
    s.constant = bool_attr(e, "constant", false);
    s.initial_level = opt_f64_attr(e, "initialLevel")?;
    Ok(s)
}

fn parse_qual_transition_list(
    reader: &mut NsReader<&[u8]>,
    model: &mut Model,
) -> Result<(), SbmlError> {
    let qual = model.qual.get_or_insert_with(QualModel::default);
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf)? {
            Event::Start(c) if ln_str(&c) == "transition" => {
                qual.transitions.push(parse_qual_transition(reader, &c)?)
            }
            Event::End(e) if ename(&e) == "listOfTransitions" => return Ok(()),
            Event::Eof => {
                return Err(SbmlError::MathParse("unexpected EOF in listOfTransitions".into()))
            }
            _ => {}
        }
    }
}

fn parse_qual_transition(
    reader: &mut NsReader<&[u8]>,
    e: &BytesStart,
) -> Result<QualTransition, SbmlError> {
    let mut t = QualTransition::default();
    t.id = attr_opt(e, "id");
    t.name = attr_opt(e, "name");
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf)? {
            Event::Start(c) => match ln_str(&c).as_str() {
                "listOfInputs" => parse_qual_inputs(reader, &mut t.inputs)?,
                "listOfOutputs" => parse_qual_outputs(reader, &mut t.outputs)?,
                "defaultTerm" | "functionTerm" => {
                    let mut mbuf = Vec::new();
                    loop {
                        match reader.read_event_into(&mut mbuf)? {
                            Event::Start(cc) if ln_str(&cc) == "math" => {
                                t.function = Some(parse_math(reader)?)
                            }
                            Event::End(end)
                                if ename(&end) == "defaultTerm"
                                    || ename(&end) == "functionTerm" =>
                            {
                                break
                            }
                            Event::Eof => {
                                return Err(SbmlError::MathParse("unexpected EOF".into()))
                            }
                            _ => {}
                        }
                    }
                }
                _ => {}
            },
            Event::End(end) if ename(&end) == "transition" => return Ok(t),
            Event::Eof => return Err(SbmlError::MathParse("unexpected EOF in transition".into())),
            _ => {}
        }
    }
}

fn parse_qual_inputs(
    reader: &mut NsReader<&[u8]>,
    out: &mut Vec<QualInput>,
) -> Result<(), SbmlError> {
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf)? {
            Event::Start(c) if ln_str(&c) == "input" => {
                let mut i = QualInput::default();
                i.qualitative_species = req_attr(&c, "qualitativeSpecies")?;
                i.sign = attr_opt(&c, "sign");
                i.transition_effect = attr_opt(&c, "transitionEffect");
                out.push(i);
                skip_to_end(reader, "input")?;
            }
            Event::Empty(c) if ln_str(&c) == "input" => {
                let mut i = QualInput::default();
                i.qualitative_species = req_attr(&c, "qualitativeSpecies")?;
                i.sign = attr_opt(&c, "sign");
                i.transition_effect = attr_opt(&c, "transitionEffect");
                out.push(i);
            }
            Event::End(e) if ename(&e) == "listOfInputs" => return Ok(()),
            Event::Eof => {
                return Err(SbmlError::MathParse("unexpected EOF in listOfInputs".into()))
            }
            _ => {}
        }
    }
}

fn parse_qual_outputs(
    reader: &mut NsReader<&[u8]>,
    out: &mut Vec<QualOutput>,
) -> Result<(), SbmlError> {
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf)? {
            Event::Start(c) if ln_str(&c) == "output" => {
                let mut o = QualOutput::default();
                o.qualitative_species = req_attr(&c, "qualitativeSpecies")?;
                o.transition_effect = attr_opt(&c, "transitionEffect");
                o.output_level = opt_f64_attr(&c, "outputLevel")?;
                out.push(o);
                skip_to_end(reader, "output")?;
            }
            Event::Empty(c) if ln_str(&c) == "output" => {
                let mut o = QualOutput::default();
                o.qualitative_species = req_attr(&c, "qualitativeSpecies")?;
                o.transition_effect = attr_opt(&c, "transitionEffect");
                o.output_level = opt_f64_attr(&c, "outputLevel")?;
                out.push(o);
            }
            Event::End(e) if ename(&e) == "listOfOutputs" => return Ok(()),
            Event::Eof => {
                return Err(SbmlError::MathParse("unexpected EOF in listOfOutputs".into()))
            }
            _ => {}
        }
    }
}

fn parse_spatial_geometry(
    reader: &mut NsReader<&[u8]>,
    e: &BytesStart,
    model: &mut Model,
) -> Result<(), SbmlError> {
    let mut g = SpatialGeometry::default();
    g.id = attr_opt(e, "id").unwrap_or_default();
    g.dimensions = opt_i64_attr(e, "dimensions")?.map(|v| v as u32);
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf)? {
            Event::Start(c) => match ln_str(&c).as_str() {
                "listOfCoordinateSystems" => {
                    let mut inner = Vec::new();
                    loop {
                        match reader.read_event_into(&mut inner)? {
                            Event::Start(u) if ln_str(&u) == "coordinateSystem" => {
                                if let Some(id) = attr_opt(&u, "id") {
                                    g.coordinate_systems.push(id);
                                }
                                skip_to_end(reader, "coordinateSystem")?;
                            }
                            Event::Empty(u) if ln_str(&u) == "coordinateSystem" => {
                                if let Some(id) = attr_opt(&u, "id") {
                                    g.coordinate_systems.push(id);
                                }
                            }
                            Event::End(end) if ename(&end) == "listOfCoordinateSystems" => break,
                            Event::Eof => {
                                return Err(SbmlError::MathParse("unexpected EOF".into()))
                            }
                            _ => {}
                        }
                    }
                }
                "listOfDomainTypes" => {
                    let mut inner = Vec::new();
                    loop {
                        match reader.read_event_into(&mut inner)? {
                            Event::Start(u) if ln_str(&u) == "domainType" => {
                                if let Some(id) = attr_opt(&u, "id") {
                                    g.domain_types.push(id);
                                }
                                skip_to_end(reader, "domainType")?;
                            }
                            Event::Empty(u) if ln_str(&u) == "domainType" => {
                                if let Some(id) = attr_opt(&u, "id") {
                                    g.domain_types.push(id);
                                }
                            }
                            Event::End(end) if ename(&end) == "listOfDomainTypes" => break,
                            Event::Eof => {
                                return Err(SbmlError::MathParse("unexpected EOF".into()))
                            }
                            _ => {}
                        }
                    }
                }
                "listOfDomains" => {
                    let mut inner = Vec::new();
                    loop {
                        match reader.read_event_into(&mut inner)? {
                            Event::Start(u) if ln_str(&u) == "domain" => {
                                if let Some(id) = attr_opt(&u, "id") {
                                    g.domains.push(id);
                                }
                                skip_to_end(reader, "domain")?;
                            }
                            Event::Empty(u) if ln_str(&u) == "domain" => {
                                if let Some(id) = attr_opt(&u, "id") {
                                    g.domains.push(id);
                                }
                            }
                            Event::End(end) if ename(&end) == "listOfDomains" => break,
                            Event::Eof => {
                                return Err(SbmlError::MathParse("unexpected EOF".into()))
                            }
                            _ => {}
                        }
                    }
                }
                "listOfGeometryDefinitions" => {
                    let mut inner = Vec::new();
                    loop {
                        match reader.read_event_into(&mut inner)? {
                            Event::Start(u) if ln_str(&u) == "geometryDefinition" => {
                                if let Some(id) = attr_opt(&u, "id") {
                                    g.geometry_definitions.push(id);
                                }
                                skip_to_end(reader, "geometryDefinition")?;
                            }
                            Event::Empty(u) if ln_str(&u) == "geometryDefinition" => {
                                if let Some(id) = attr_opt(&u, "id") {
                                    g.geometry_definitions.push(id);
                                }
                            }
                            Event::End(end) if ename(&end) == "listOfGeometryDefinitions" => break,
                            Event::Eof => {
                                return Err(SbmlError::MathParse("unexpected EOF".into()))
                            }
                            _ => {}
                        }
                    }
                }
                _ => {}
            },
            Event::End(end) if ename(&end) == "geometry" => {
                model.spatial.get_or_insert_with(SpatialModel::default).geometry = Some(g);
                return Ok(());
            }
            Event::Eof => return Err(SbmlError::MathParse("unexpected EOF in geometry".into())),
            _ => {}
        }
    }
}

// ---------------------------------------------------------------------------
// `Reader` trait integration
// ---------------------------------------------------------------------------

/// Default SBML reader: parses a whole buffer into a [`Sbml`] model.
#[cfg(feature = "alloc")]
#[derive(Debug, Clone, Default)]
pub struct SbmlParser {
    /// Reserved for future validation settings; currently parsing is strict.
    pub settings: ValidationSettings,
}

#[cfg(feature = "alloc")]
impl Reader<Sbml> for SbmlParser {
    type Error = SbmlError;
    fn parse(&self, input: &[u8]) -> Result<Sbml, SbmlError> {
        parse_sbml(input)
    }
}
