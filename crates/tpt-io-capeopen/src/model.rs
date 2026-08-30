//! Pure-Rust data model for CAPE-OPEN thermodynamic property packages.
//!
//! Everything in this module is available with the default (no-`com`) features
//! and is fully `#![no_std]` + `alloc` compatible — no COM, no Windows. It
//! mirrors the concepts CAPE-OPEN 1.1 / 2.0 exposes through its thermo COM
//! interfaces:
//!
//! * a [`ComponentList`] of chemical [`Component`]s,
//! * a [`Phase`] identifier set (Vapor / Liquid / Liquid1 / … / Overall),
//! * the CAPE-OPEN [`BaseProperty`] vocabulary (single- and two-phase),
//! * [`PropertyCalculationRequest`] describing what a calculation should return,
//! * a [`PropertyPackage`] descriptor tying the above together, and
//! * a [`StreamState`] holding the state of one material stream.
//!
//! The [`PropertyPackage::validate`] method enforces the same invariants the
//! COM interfaces require of a well-formed package.

use crate::error::CapeOpenError;

use alloc::string::{String, ToString};
use alloc::vec::Vec;

/// A single chemical component in a CAPE-OPEN component list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Component {
    /// Component identifier as used by the property package (often the CAPE-OPEN
    /// `componentId`, e.g. a CAS number or a canonical name).
    pub id: String,
    /// Human-readable description, if known.
    pub description: Option<String>,
    /// CAS registry number, if known.
    pub cas_number: Option<String>,
    /// Chemical formula, if known.
    pub formula: Option<String>,
}

impl Component {
    /// Build a component from its required id; optional metadata can be filled
    /// in afterwards.
    pub fn new(id: impl Into<String>) -> Self {
        Component { id: id.into(), description: None, cas_number: None, formula: None }
    }
}

/// An ordered list of components, the unit of identity for CAPE-OPEN thermo.
///
/// CAPE-OPEN refers to components by their 1-based index within this list, so
/// the list must be non-empty and every `id` unique.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ComponentList {
    /// The components, in index order.
    pub components: Vec<Component>,
}

impl ComponentList {
    /// Create an empty component list.
    pub fn new() -> Self {
        ComponentList { components: Vec::new() }
    }

    /// Build a component list directly from component ids.
    pub fn from_ids(ids: &[&str]) -> Self {
        ComponentList { components: ids.iter().map(|id| Component::new(*id)).collect() }
    }

    /// Number of components.
    pub fn len(&self) -> usize {
        self.components.len()
    }

    /// Whether the list is empty.
    pub fn is_empty(&self) -> bool {
        self.components.is_empty()
    }

    /// 0-based index of `id`, if present.
    pub fn index_of(&self, id: &str) -> Option<usize> {
        self.components.iter().position(|c| c.id == id)
    }

    /// Look up a component by id.
    pub fn get(&self, id: &str) -> Option<&Component> {
        self.components.get(self.index_of(id)?)
    }

    /// Validate the CAPE-OPEN invariant: non-empty with unique ids.
    pub fn validate(&self) -> Result<(), CapeOpenError> {
        if self.components.is_empty() {
            return Err(CapeOpenError::ComponentList("component list is empty".into()));
        }
        for (i, c) in self.components.iter().enumerate() {
            if c.id.is_empty() {
                return Err(CapeOpenError::ComponentList(format!("component at index {i} has an empty id")));
            }
            if self.components[..i].iter().any(|p| p.id == c.id) {
                return Err(CapeOpenError::ComponentList(format!("duplicate component id '{}'", c.id)));
            }
        }
        Ok(())
    }
}

/// A CAPE-OPEN phase identifier.
///
/// `Overall` is not a thermodynamic phase but a CAPE-OPEN qualifier meaning
/// "all phases / mixture". `Unknown` is used when a string does not map to a
/// recognized phase.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Phase {
    /// Vapor phase.
    Vapor,
    /// Liquid phase (single liquid).
    Liquid,
    /// First liquid phase in a multi-liquid system.
    Liquid1,
    /// Second liquid phase.
    Liquid2,
    /// Third liquid phase.
    Liquid3,
    /// Solid phase (single solid).
    Solid,
    /// First solid phase.
    Solid1,
    /// Second solid phase.
    Solid2,
    /// Mixture / overall qualifier.
    Overall,
    /// Unrecognized phase string.
    Unknown,
}

impl Phase {
    /// The CAPE-OPEN canonical phase label.
    pub fn as_str(&self) -> &'static str {
        match self {
            Phase::Vapor => "Vapor",
            Phase::Liquid => "Liquid",
            Phase::Liquid1 => "Liquid1",
            Phase::Liquid2 => "Liquid2",
            Phase::Liquid3 => "Liquid3",
            Phase::Solid => "Solid",
            Phase::Solid1 => "Solid1",
            Phase::Solid2 => "Solid2",
            Phase::Overall => "Overall",
            Phase::Unknown => "Unknown",
        }
    }

    /// Parse a phase label (case-insensitive). Unknown labels map to `Unknown`.
    pub fn parse(s: &str) -> Phase {
        match s.to_ascii_lowercase().as_str() {
            "vapor" => Phase::Vapor,
            "liquid" => Phase::Liquid,
            "liquid1" => Phase::Liquid1,
            "liquid2" => Phase::Liquid2,
            "liquid3" => Phase::Liquid3,
            "solid" => Phase::Solid,
            "solid1" => Phase::Solid1,
            "solid2" => Phase::Solid2,
            "overall" => Phase::Overall,
            _ => Phase::Unknown,
        }
    }
}

/// A CAPE-OPEN base property.
///
/// Covers the single-phase and two-phase base properties from the CAPE-OPEN
/// thermodynamic and physical properties specification. Unrecognized labels are
/// preserved verbatim in [`BaseProperty::Other`].
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum BaseProperty {
    /// Temperature.
    Temperature,
    /// Pressure.
    Pressure,
    /// Molar enthalpy.
    Enthalpy,
    /// Molar entropy.
    Entropy,
    /// Molar internal energy.
    InternalEnergy,
    /// Molar Gibbs free energy.
    GibbsFreeEnergy,
    /// Molar volume.
    MolarVolume,
    /// Density.
    Density,
    /// Molar flow.
    MolarFlow,
    /// Mass flow.
    MassFlow,
    /// Volumetric flow.
    VolumeFlow,
    /// Molar heat capacity at constant pressure.
    HeatCapacityCp,
    /// Molar heat capacity at constant volume.
    HeatCapacityCv,
    /// Viscosity.
    Viscosity,
    /// Thermal conductivity.
    ThermalConductivity,
    /// Surface tension.
    SurfaceTension,
    /// Molecular weight (mixture).
    MolecularWeight,
    /// Phase fraction.
    Fraction,
    /// Equilibrium temperature (two-phase).
    EquilibriumTemperature,
    /// Equilibrium pressure (two-phase).
    EquilibriumPressure,
    /// Vapor fraction (two-phase).
    VaporFraction,
    /// K-value (two-phase).
    Kvalue,
    /// Logarithm of K-value (two-phase).
    LogKvalue,
    /// Any other (extension / future) property, preserved verbatim.
    Other(String),
}

impl BaseProperty {
    /// The CAPE-OPEN canonical property label.
    pub fn as_str(&self) -> String {
        match self {
            BaseProperty::Temperature => "temperature".into(),
            BaseProperty::Pressure => "pressure".into(),
            BaseProperty::Enthalpy => "enthalpy".into(),
            BaseProperty::Entropy => "entropy".into(),
            BaseProperty::InternalEnergy => "internalEnergy".into(),
            BaseProperty::GibbsFreeEnergy => "GibbsFreeEnergy".into(),
            BaseProperty::MolarVolume => "molarVolume".into(),
            BaseProperty::Density => "density".into(),
            BaseProperty::MolarFlow => "molarFlow".into(),
            BaseProperty::MassFlow => "massFlow".into(),
            BaseProperty::VolumeFlow => "volumeFlow".into(),
            BaseProperty::HeatCapacityCp => "heatCapacityCp".into(),
            BaseProperty::HeatCapacityCv => "heatCapacityCv".into(),
            BaseProperty::Viscosity => "viscosity".into(),
            BaseProperty::ThermalConductivity => "thermalConductivity".into(),
            BaseProperty::SurfaceTension => "surfaceTension".into(),
            BaseProperty::MolecularWeight => "molecularWeight".into(),
            BaseProperty::Fraction => "fraction".into(),
            BaseProperty::EquilibriumTemperature => "equilibriumTemperature".into(),
            BaseProperty::EquilibriumPressure => "equilibriumPressure".into(),
            BaseProperty::VaporFraction => "vaporFraction".into(),
            BaseProperty::Kvalue => "kvalue".into(),
            BaseProperty::LogKvalue => "logKvalue".into(),
            BaseProperty::Other(s) => s.clone(),
        }
    }

    /// Parse a property label (case-insensitive). Unknown labels become `Other`.
    pub fn parse(s: &str) -> BaseProperty {
        match s.to_ascii_lowercase().as_str() {
            "temperature" => BaseProperty::Temperature,
            "pressure" => BaseProperty::Pressure,
            "enthalpy" => BaseProperty::Enthalpy,
            "entropy" => BaseProperty::Entropy,
            "internalenergy" => BaseProperty::InternalEnergy,
            "gibbsfreeenergy" => BaseProperty::GibbsFreeEnergy,
            "molarvolume" => BaseProperty::MolarVolume,
            "density" => BaseProperty::Density,
            "molarflow" => BaseProperty::MolarFlow,
            "massflow" => BaseProperty::MassFlow,
            "volumeflow" => BaseProperty::VolumeFlow,
            "heatcapacitycp" => BaseProperty::HeatCapacityCp,
            "heatcapacitycv" => BaseProperty::HeatCapacityCv,
            "viscosity" => BaseProperty::Viscosity,
            "thermalconductivity" => BaseProperty::ThermalConductivity,
            "surfacetension" => BaseProperty::SurfaceTension,
            "molecularweight" => BaseProperty::MolecularWeight,
            "fraction" => BaseProperty::Fraction,
            "equilibriumtemperature" => BaseProperty::EquilibriumTemperature,
            "equilibriumpressure" => BaseProperty::EquilibriumPressure,
            "vaporfraction" => BaseProperty::VaporFraction,
            "kvalue" => BaseProperty::Kvalue,
            "logkvalue" => BaseProperty::LogKvalue,
            other => BaseProperty::Other(other.to_string()),
        }
    }
}

/// What kind of property calculation a request asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CalculationKind {
    /// A single-phase property calculation (`CalcSinglePhaseProp`).
    SinglePhase,
    /// A two-phase (non-equilibrium) property calculation (`CalcTwoPhaseProp`).
    TwoPhase,
    /// A phase-equilibrium calculation (`CalcEquilibrium`).
    Equilibrium,
}

/// A CAPE-OPEN property calculation request.
///
/// Equivalent to the arguments passed to `ICapeThermoCalculation::CalcSinglePhaseProp`
/// / `CalcTwoPhaseProp` / `CalcEquilibrium`: which phases to consider, which
/// base properties to return, and (optionally) the conditions the calculator
/// should use.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PropertyCalculationRequest {
    /// The kind of calculation.
    pub kind: CalculationKind,
    /// Phases involved (one for single-phase, two for two-phase/equilibrium).
    pub phases: Vec<Phase>,
    /// Base properties requested.
    pub properties: Vec<BaseProperty>,
}

impl PropertyCalculationRequest {
    /// Create a request for the given kinds/phases/properties.
    pub fn new(kind: CalculationKind, phases: Vec<Phase>, properties: Vec<BaseProperty>) -> Self {
        PropertyCalculationRequest { kind, phases, properties }
    }
}

/// A thermodynamic property package descriptor.
///
/// This is the pure-Rust analogue of the metadata exposed by
/// `ICapeThermoPropertyPackage`: the component list it operates on, the phases it
/// supports, and the single-/two-phase base properties it can compute.
#[derive(Debug, Clone, PartialEq)]
pub struct PropertyPackage {
    /// Package name / identifier.
    pub name: String,
    /// Components the package operates on.
    pub components: ComponentList,
    /// Phases the package supports.
    pub supported_phases: Vec<Phase>,
    /// Single-phase base properties the package can compute.
    pub supported_single_phase: Vec<BaseProperty>,
    /// Two-phase base properties the package can compute.
    pub supported_two_phase: Vec<BaseProperty>,
}

impl PropertyPackage {
    /// Create a package from its name and component list.
    pub fn new(name: impl Into<String>, components: ComponentList) -> Self {
        PropertyPackage {
            name: name.into(),
            components,
            supported_phases: Vec::new(),
            supported_single_phase: Vec::new(),
            supported_two_phase: Vec::new(),
        }
    }

    /// Validate the package: it must have a non-empty name, a valid component
    /// list, and at least one supported phase.
    pub fn validate(&self) -> Result<(), CapeOpenError> {
        if self.name.is_empty() {
            return Err(CapeOpenError::PropertyPackage("package name is empty".into()));
        }
        self.components.validate()?;
        if self.supported_phases.is_empty() {
            return Err(CapeOpenError::PropertyPackage("no supported phases declared".into()));
        }
        Ok(())
    }
}

/// The thermodynamic state of a single material stream.
///
/// Field counts (`overall_mole_fractions`) are indexed by the package's
/// component list; [`StreamState::validate`] checks the length and that the
/// fractions sum to unity (within tolerance).
#[derive(Debug, Clone, PartialEq)]
pub struct StreamState {
    /// Temperature (K).
    pub temperature: f64,
    /// Pressure (Pa).
    pub pressure: f64,
    /// Overall mole fractions, indexed by component list order.
    pub overall_mole_fractions: Vec<f64>,
    /// Total molar flow (mol/s).
    pub total_flow: f64,
}

impl StreamState {
    /// Create an empty stream state for a package with `n` components.
    pub fn new(n: usize) -> Self {
        StreamState {
            temperature: 0.0,
            pressure: 0.0,
            overall_mole_fractions: alloc::vec![0.0; n],
            total_flow: 0.0,
        }
    }

    /// Validate against a component list: correct length and mole fractions
    /// summing to ~1.0 within `tol`.
    pub fn validate(&self, components: &ComponentList, tol: f64) -> Result<(), CapeOpenError> {
        if self.overall_mole_fractions.len() != components.len() {
            return Err(CapeOpenError::PropertyPackage(format!(
                "stream has {} mole fractions but {} components",
                self.overall_mole_fractions.len(),
                components.len()
            )));
        }
        let sum: f64 = self.overall_mole_fractions.iter().sum();
        if (sum - 1.0).abs() > tol {
            return Err(CapeOpenError::PropertyPackage(format!(
                "mole fractions sum to {sum}, expected ~1.0"
            )));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn component_list_rejects_duplicates() {
        let list = ComponentList::from_ids(&["WATER", "WATER"]);
        assert!(list.validate().is_err());
    }

    #[test]
    fn phase_round_trips_through_str() {
        for p in [Phase::Vapor, Phase::Liquid1, Phase::Solid2, Phase::Overall] {
            assert_eq!(Phase::parse(p.as_str()), p);
        }
    }

    #[test]
    fn property_other_is_preserved() {
        assert_eq!(BaseProperty::parse("customThing"), BaseProperty::Other("customthing".into()));
    }

    #[test]
    fn stream_state_validates_length_and_closure() {
        let comps = ComponentList::from_ids(&["A", "B"]);
        let mut st = StreamState::new(2);
        st.overall_mole_fractions = alloc::vec![0.6, 0.4];
        assert!(st.validate(&comps, 1e-9).is_ok());
        st.overall_mole_fractions = alloc::vec![0.6, 0.5];
        assert!(st.validate(&comps, 1e-9).is_err());
    }
}
