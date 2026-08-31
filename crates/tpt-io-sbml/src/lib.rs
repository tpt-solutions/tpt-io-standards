//! SBML parser for the `tpt-io` workspace.
//!
//! Parses SBML Level 2 and Level 3 documents into a strongly-typed [`model`],
//! with MathML expression extraction ([`mathml`]), rule/event/function
//! definitions, the FBC / Qual / Spatial package extensions, and
//! syntactic/semantic validation with charge- and element-balance consistency
//! checking ([`validate`]).
//!
//! ```
//! # #[cfg(feature = "std")]
//! # {
//! use tpt_io_sbml::{Sbml, SbmlParser};
//! use tpt_io_core::Reader;
//!
//! let doc = br#"<?xml version="1.0"?>
//! <sbml xmlns="http://www.sbml.org/sbml/level2/version4" level="2" version="4">
//!   <model id="m">
//!     <listOfSpecies>
//!       <species id="S" compartment="c" initialConcentration="1"/>
//!     </listOfSpecies>
//!   </model>
//! </sbml>"#;
//! let sbml = Sbml::parse(doc).unwrap();
//! assert_eq!(sbml.level, 2);
//! assert_eq!(sbml.model.species.len(), 1);
//! # }
//! ```

#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]
// The parser incrementally builds typed structs via `X::default()` followed by
// field assignments; this stylistic lint is impractical to satisfy here.
#![allow(clippy::field_reassign_with_default)]

#[cfg(any(feature = "alloc", feature = "std"))]
extern crate alloc;

pub mod error;
pub mod integration;
pub mod mathml;
pub mod model;
pub mod parser;
pub mod validate;

pub use error::SbmlError;
pub use model::{
    Compartment, Constraint, Event, EventAssignment, FbcModel, FunctionDefinition,
    InitialAssignment, KineticLaw, Model, ModifierSpeciesReference, Objective, Parameter,
    QualModel, QualSpecies, Reaction, Rule, Sbml, Species, SpeciesReference, Trigger, Unit,
    UnitDefinition,
};

pub use parser::{parse_sbml, SbmlParser};

pub use validate::{
    check_mass_balance, check_semantic, check_syntactic, validate_sbml, ValidationIssue,
    ValidationReport,
};

#[cfg(any(feature = "alloc", feature = "std"))]
pub use tpt_io_core::ParseError;
pub use tpt_io_core::{
    Encoding, FormatExtension, NoProgress, Progress, ProgressFn, Reader, Schema,
    ValidationSettings, Validator, Writer,
};

#[cfg(feature = "alloc")]
pub use mathml::MathNode;

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = br#"<?xml version="1.0" encoding="UTF-8"?>
<sbml xmlns="http://www.sbml.org/sbml/level2/version4" level="2" version="4">
  <model id="branchModel" name="Branch">
    <listOfUnitDefinitions>
      <unitDefinition id="substance">
        <listOfUnits>
          <unit kind="mole" scale="-3" exponent="1"/>
        </listOfUnits>
      </unitDefinition>
    </listOfUnitDefinitions>
    <listOfCompartments>
      <compartment id="c" size="1" constant="true"/>
    </listOfCompartments>
    <listOfSpecies>
      <species id="S1" compartment="c" initialConcentration="1" charge="0"/>
      <species id="S2" compartment="c" initialConcentration="0" charge="0"/>
    </listOfSpecies>
    <listOfReactions>
      <reaction id="R1" reversible="false" fast="false">
        <listOfReactants>
          <speciesReference species="S1" stoichiometry="1"/>
        </listOfReactants>
        <listOfProducts>
          <speciesReference species="S2" stoichiometry="1"/>
        </listOfProducts>
        <kineticLaw>
          <math xmlns="http://www.w3.org/1998/Math/MathML">
            <apply>
              <times/>
              <ci>k1</ci>
              <ci>S1</ci>
            </apply>
          </math>
          <listOfParameters>
            <parameter id="k1" value="0.5"/>
          </listOfParameters>
        </kineticLaw>
      </reaction>
    </listOfReactions>
  </model>
</sbml>"#;

    #[test]
    fn parse_sample() {
        let sbml = Sbml::parse(SAMPLE).expect("parsed");
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
    }

    #[test]
    fn reader_trait() {
        let parser = SbmlParser::default();
        let sbml = parser.parse(SAMPLE).expect("parsed via Reader");
        assert_eq!(sbml.model.species.len(), 2);
    }

    #[test]
    fn validate_sample() {
        let sbml = Sbml::parse(SAMPLE).unwrap();
        let report = validate::validate_sbml(&sbml);
        assert!(report.is_ok(), "issues: {:?}", report.errors);
    }
}
