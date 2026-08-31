//! Strongly-typed data model for parsed SBML documents.
//!
//! Covers the SBML core (Levels 2 and 3): model, compartments, species,
//! parameters, reactions (with kinetic laws), rules, initial assignments,
//! constraints, events, function definitions, and unit definitions. Package
//! extensions (FBC, Qual, Spatial) are attached under optional fields.

use alloc::string::String;
use alloc::vec::Vec;

use crate::mathml::MathNode;

/// A fully parsed SBML document.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Sbml {
    /// SBML Level (2 or 3).
    pub level: u32,
    /// SBML Version within the Level.
    pub version: u32,
    /// The single top-level model.
    pub model: Model,
}

/// The top-level SBML model container.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Model {
    /// Model `id` (SId).
    pub id: Option<String>,
    /// Model `name` (optional human-readable).
    pub name: Option<String>,
    /// Default units for substances.
    pub substance_units: Option<String>,
    /// Default units for time.
    pub time_units: Option<String>,
    /// Default units for volume.
    pub volume_units: Option<String>,
    /// Default units for area.
    pub area_units: Option<String>,
    /// Default units for length.
    pub length_units: Option<String>,
    /// Default units for extents.
    pub extent_units: Option<String>,
    /// `conversionFactor` SId (Level 3).
    pub conversion_factor: Option<String>,

    pub unit_definitions: Vec<UnitDefinition>,
    pub compartments: Vec<Compartment>,
    pub species: Vec<Species>,
    pub parameters: Vec<Parameter>,
    pub initial_assignments: Vec<InitialAssignment>,
    pub rules: Vec<Rule>,
    pub constraints: Vec<Constraint>,
    pub reactions: Vec<Reaction>,
    pub events: Vec<Event>,
    pub function_definitions: Vec<FunctionDefinition>,

    /// FBC (flux balance analysis) package data, if the model uses the extension.
    pub fbc: Option<FbcModel>,
    /// Qual (qualitative models) package data, if present.
    pub qual: Option<QualModel>,
    /// Spatial package data, if present.
    pub spatial: Option<SpatialModel>,
}

impl Model {
    /// Look up a compartment by `id`.
    pub fn compartment_by_id(&self, id: &str) -> Option<&Compartment> {
        self.compartments.iter().find(|c| c.id == id)
    }
    /// Look up a species by `id`.
    pub fn species_by_id(&self, id: &str) -> Option<&Species> {
        self.species.iter().find(|s| s.id == id)
    }
    /// Look up a parameter by `id`.
    pub fn parameter_by_id(&self, id: &str) -> Option<&Parameter> {
        self.parameters.iter().find(|p| p.id == id)
    }
    /// Look up a reaction by `id`.
    pub fn reaction_by_id(&self, id: &str) -> Option<&Reaction> {
        self.reactions.iter().find(|r| r.id == id)
    }
    /// Look up a function definition by `id`.
    pub fn function_definition_by_id(&self, id: &str) -> Option<&FunctionDefinition> {
        self.function_definitions.iter().find(|f| f.id == id)
    }
    /// Look up a unit definition by `id`.
    pub fn unit_definition_by_id(&self, id: &str) -> Option<&UnitDefinition> {
        self.unit_definitions.iter().find(|u| u.id == id)
    }

    /// Return every identifier that may be referenced from MathML `ci`/`csymbol`
    /// expressions within this model (species, parameters, compartments,
    /// reactions for L3, function definitions, and local parameters elsewhere).
    pub fn known_identifiers(&self) -> Vec<String> {
        let mut ids = Vec::new();
        for c in &self.compartments {
            ids.push(c.id.clone());
        }
        for s in &self.species {
            ids.push(s.id.clone());
        }
        for p in &self.parameters {
            ids.push(p.id.clone());
        }
        for r in &self.reactions {
            ids.push(r.id.clone());
        }
        for f in &self.function_definitions {
            ids.push(f.id.clone());
        }
        ids
    }
}

/// A `<unitDefinition>`: a named collection of [`Unit`]s.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct UnitDefinition {
    /// Unit definition `id` (SId).
    pub id: String,
    /// Optional `name`.
    pub name: Option<String>,
    /// Constituent units.
    pub units: Vec<Unit>,
}

/// A single unit term within a [`UnitDefinition`].
#[derive(Debug, Clone, PartialEq)]
pub struct Unit {
    /// Base unit kind (e.g. `"mole"`, `"litre"`, `"second"`).
    pub kind: String,
    /// Exponent (default `1`).
    pub exponent: f64,
    /// Scale (power of ten, default `0`).
    pub scale: i64,
    /// Multiplier (default `1`).
    pub multiplier: f64,
}

impl Default for Unit {
    fn default() -> Self {
        Unit { kind: String::new(), exponent: 1.0, scale: 0, multiplier: 1.0 }
    }
}

/// A `<compartment>`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Compartment {
    /// Compartment `id`.
    pub id: String,
    /// Optional `name`.
    pub name: Option<String>,
    /// Spatial dimensions (1, 2, or 3).
    pub spatial_dimensions: Option<f64>,
    /// Size/volume of the compartment.
    pub size: Option<f64>,
    /// Units SId for this compartment's size.
    pub units: Option<String>,
    /// Whether the compartment size can vary (SBML default `true`).
    pub constant: bool,
    /// `outside` compartment SId, if nested.
    pub outside: Option<String>,
}

/// A `<species>`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Species {
    /// Species `id`.
    pub id: String,
    /// Optional `name`.
    pub name: Option<String>,
    /// Compartment SId this species resides in.
    pub compartment: String,
    /// Initial concentration (Level 2/3).
    pub initial_concentration: Option<f64>,
    /// Initial amount (Level 2/3).
    pub initial_amount: Option<f64>,
    /// Substance units SId.
    pub substance_units: Option<String>,
    /// Whether the species is described only by substance (not concentration).
    pub has_only_substance_units: bool,
    /// Boundary condition flag.
    pub boundary_condition: bool,
    /// Whether the species quantity is constant.
    pub constant: bool,
    /// Charge (Level 2 attribute).
    pub charge: Option<i64>,
    /// Charge from the FBC package (`fbc:charge`).
    pub fbc_charge: Option<i64>,
    /// Chemical formula from the FBC package (`fbc:chemicalFormula`).
    pub chemical_formula: Option<String>,
}

/// A `<parameter>` (model-level or local to a kinetic law).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Parameter {
    /// Parameter `id`.
    pub id: String,
    /// Optional `name`.
    pub name: Option<String>,
    /// Numeric value.
    pub value: Option<f64>,
    /// Units SId.
    pub units: Option<String>,
    /// Whether the parameter is constant (SBML default `true`).
    pub constant: bool,
}

/// A reactant or product reference inside a reaction.
#[derive(Debug, Clone, PartialEq)]
pub struct SpeciesReference {
    /// Optional `id` (Level 3).
    pub id: Option<String>,
    /// Optional `name`.
    pub name: Option<String>,
    /// Referenced species SId.
    pub species: String,
    /// Stoichiometry (default `1`).
    pub stoichiometry: f64,
    /// Whether the stoichiometry is constant.
    pub constant: bool,
}

impl Default for SpeciesReference {
    fn default() -> Self {
        SpeciesReference {
            id: None,
            name: None,
            species: String::new(),
            stoichiometry: 1.0,
            constant: true,
        }
    }
}

/// A modifier species reference (`<modifierSpeciesReference>`).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ModifierSpeciesReference {
    /// Optional `id`.
    pub id: Option<String>,
    /// Optional `name`.
    pub name: Option<String>,
    /// Referenced species SId.
    pub species: String,
}

/// A `<kineticLaw>` for a reaction.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct KineticLaw {
    /// The parsed math expression (if present).
    pub math: Option<MathNode>,
    /// Local parameters defined within the kinetic law.
    pub parameters: Vec<Parameter>,
    /// Substance units SId for the law.
    pub substance_units: Option<String>,
    /// Time units SId for the law.
    pub time_units: Option<String>,
}

/// A `<reaction>`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Reaction {
    /// Reaction `id`.
    pub id: String,
    /// Optional `name`.
    pub name: Option<String>,
    /// Whether the reaction is reversible (SBML default `true`).
    pub reversible: bool,
    /// Whether the reaction is fast (SBML default `false`).
    pub fast: bool,
    /// Reactant species references.
    pub reactants: Vec<SpeciesReference>,
    /// Product species references.
    pub products: Vec<SpeciesReference>,
    /// Modifier species references.
    pub modifiers: Vec<ModifierSpeciesReference>,
    /// Kinetic law.
    pub kinetic_law: Option<KineticLaw>,
    /// FBC `fbc:lowerFluxBound` (reaction-level flux bound reference).
    pub lower_flux_bound: Option<String>,
    /// FBC `fbc:upperFluxBound` (reaction-level flux bound reference).
    pub upper_flux_bound: Option<String>,
    /// FBC `fbc:geneProductAssociation` (GPR string).
    pub gene_product_association: Option<String>,
}

/// A rule: assignment, rate, or algebraic.
#[derive(Debug, Clone, PartialEq)]
pub enum Rule {
    /// `<assignmentRule>`: `variable = math`.
    Assignment {
        /// Target SId.
        variable: String,
        /// Assigning expression.
        math: MathNode,
    },
    /// `<rateRule>`: `d(variable)/dt = math`.
    Rate {
        /// Target SId.
        variable: String,
        /// Rate expression.
        math: MathNode,
    },
    /// `<algebraicRule>`: `math = 0`.
    Algebraic {
        /// Optional `variable` (Level 2).
        variable: Option<String>,
        /// Constraint expression (equals zero).
        math: MathNode,
    },
}

/// An `<initialAssignment>`: sets the initial value of a symbol.
#[derive(Debug, Clone, PartialEq)]
pub struct InitialAssignment {
    /// Target symbol SId.
    pub symbol: String,
    /// Value expression.
    pub math: MathNode,
}

/// A `<constraint>`: a model-wide invariant.
#[derive(Debug, Clone, PartialEq)]
pub struct Constraint {
    /// Constraint expression (must evaluate true).
    pub math: MathNode,
    /// Optional XHTML message.
    pub message: Option<String>,
}

/// A `<trigger>` for an event.
#[derive(Debug, Clone, PartialEq)]
pub struct Trigger {
    /// Trigger Boolean expression.
    pub math: MathNode,
    /// `initialValue` (Level 3).
    pub initial_value: Option<bool>,
    /// `persistent` (Level 3).
    pub persistent: Option<bool>,
}

/// An `<eventAssignment>` within an event.
#[derive(Debug, Clone, PartialEq)]
pub struct EventAssignment {
    /// Target variable SId.
    pub variable: String,
    /// Assignment expression.
    pub math: MathNode,
}

/// A `<event>`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Event {
    /// Event `id`.
    pub id: Option<String>,
    /// Optional `name`.
    pub name: Option<String>,
    /// Trigger.
    pub trigger: Option<Trigger>,
    /// Optional delay expression.
    pub delay: Option<MathNode>,
    /// Optional priority expression (Level 3).
    pub priority: Option<MathNode>,
    /// Time units SId.
    pub time_units: Option<String>,
    /// Event assignments.
    pub event_assignments: Vec<EventAssignment>,
}

/// A `<functionDefinition>`: a user-defined named function (lambda).
#[derive(Debug, Clone, PartialEq)]
pub struct FunctionDefinition {
    /// Function `id`.
    pub id: String,
    /// Optional `name`.
    pub name: Option<String>,
    /// Body: a `<lambda>` expression.
    pub math: MathNode,
}

// ---------------------------------------------------------------------------
// Package extensions
// ---------------------------------------------------------------------------

/// FBC (flux balance analysis) package data (`fbc:` namespace, Level 3 only).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct FbcModel {
    /// `fbc:strict` flag.
    pub strict: bool,
    /// Objectives (`listOfObjectives`).
    pub objectives: Vec<Objective>,
    /// Flux bounds (`listOfFluxBounds`).
    pub flux_bounds: Vec<FluxBound>,
    /// Gene products (`listOfGeneProducts`).
    pub gene_products: Vec<GeneProduct>,
}

/// An FBC objective (`<fbc:objective>`).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Objective {
    /// Objective `id`.
    pub id: String,
    /// Optional `name`.
    pub name: Option<String>,
    /// `"maximize"` or `"minimize"`.
    pub type_: String,
    /// Weighted flux objectives.
    pub flux_objectives: Vec<FluxObjective>,
}

/// A weighted reaction within an [`Objective`].
#[derive(Debug, Clone, PartialEq)]
pub struct FluxObjective {
    /// Target reaction SId.
    pub reaction: String,
    /// Weighting coefficient.
    pub coefficient: f64,
}

impl Default for FluxObjective {
    fn default() -> Self {
        FluxObjective { reaction: String::new(), coefficient: 1.0 }
    }
}

/// An FBC flux bound (`<fbc:fluxBound>`).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct FluxBound {
    /// Optional `id`.
    pub id: Option<String>,
    /// Target reaction SId.
    pub reaction: String,
    /// Operation (`"less"`, `"greater"`, `"lessEqual"`, `"greaterEqual"`, `"equal"`).
    pub operation: String,
    /// Bound value.
    pub value: f64,
}

/// An FBC gene product (`<fbc:geneProduct>`).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct GeneProduct {
    /// Gene product `id`.
    pub id: String,
    /// Optional `name`.
    pub name: Option<String>,
    /// Optional `label`.
    pub label: Option<String>,
    /// Associated species SId (optional).
    pub associated_species: Option<String>,
}

/// Qual (qualitative models) package data (`qual:` namespace, Level 3).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct QualModel {
    /// Qualitative species.
    pub qualitative_species: Vec<QualSpecies>,
    /// Transitions.
    pub transitions: Vec<QualTransition>,
}

/// A qualitative species (`<qual:qualitativeSpecies>`).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct QualSpecies {
    /// Species `id`.
    pub id: String,
    /// Optional `name`.
    pub name: Option<String>,
    /// Compartment SId (optional).
    pub compartment: Option<String>,
    /// Whether the level is constant.
    pub constant: bool,
    /// Initial level.
    pub initial_level: Option<f64>,
}

/// A transition input (`<qual:input>`).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct QualInput {
    /// Referenced qualitative species SId.
    pub qualitative_species: String,
    /// Sign (`"positive"`, `"negative"`, `"dual"`, etc.).
    pub sign: Option<String>,
    /// Transition effect.
    pub transition_effect: Option<String>,
}

/// A transition output (`<qual:output>`).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct QualOutput {
    /// Referenced qualitative species SId.
    pub qualitative_species: String,
    /// Transition effect.
    pub transition_effect: Option<String>,
    /// Output level (optional).
    pub output_level: Option<f64>,
}

/// A qualitative transition (`<qual:transition>`).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct QualTransition {
    /// Transition `id`.
    pub id: Option<String>,
    /// Optional `name`.
    pub name: Option<String>,
    /// Inputs.
    pub inputs: Vec<QualInput>,
    /// Outputs.
    pub outputs: Vec<QualOutput>,
    /// Default term / function expression (optional).
    pub function: Option<MathNode>,
}

/// Spatial package data (`spatial:` namespace) — parsed at a summary level.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SpatialModel {
    /// Geometry, if defined.
    pub geometry: Option<SpatialGeometry>,
}

/// A `spatial:geometry` element (summary view).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SpatialGeometry {
    /// Geometry `id`.
    pub id: String,
    /// Number of spatial dimensions.
    pub dimensions: Option<u32>,
    /// Coordinate system identifiers.
    pub coordinate_systems: Vec<String>,
    /// Domain type identifiers.
    pub domain_types: Vec<String>,
    /// Domain identifiers.
    pub domains: Vec<String>,
    /// Geometry definition identifiers.
    pub geometry_definitions: Vec<String>,
}
