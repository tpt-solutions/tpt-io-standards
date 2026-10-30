//! Property-based tests (proptest) for SBML parsing invariants.
//!
//! Focus: the parser/validator must never panic on arbitrary bytes, and parsing
//! must be deterministic for valid input. (A full writer/round-trip property is
//! out of scope until this crate gains a serializer — see `README.md`.)

use proptest::prelude::*;
use tpt_io_sbml::{parse_sbml, validate_sbml, Sbml};

/// A small, always-valid Level 2 document used for determinism checks.
const VALID_L2: &[u8] = br#"<?xml version="1.0"?>
<sbml xmlns="http://www.sbml.org/sbml/level2/version4" level="2" version="4">
  <model id="m">
    <listOfCompartments><compartment id="c" constant="true"/></listOfCompartments>
    <listOfSpecies><species id="S" compartment="c" initialConcentration="1"/></listOfSpecies>
    <listOfReactions>
      <reaction id="R">
        <listOfReactants><speciesReference species="S" stoichiometry="1"/></listOfReactants>
        <kineticLaw>
          <math xmlns="http://www.w3.org/1998/Math/MathML"><apply><ci>S</ci></apply></math>
        </kineticLaw>
      </reaction>
    </listOfReactions>
  </model>
</sbml>"#;

proptest! {
    #[test]
    fn parse_never_panics(data in proptest::collection::vec(any::<u8>(), 0..1024)) {
        // The parser must return Ok/Err, never panic, on arbitrary bytes.
        let _ = parse_sbml(&data);
    }

    #[test]
    fn parse_and_validate_never_panics(data in proptest::collection::vec(any::<u8>(), 0..1024)) {
        if let Ok(sbml) = parse_sbml(&data) {
            // Consistency checks must also be panic-free on whatever parsed.
            let _ = validate_sbml(&sbml);
        }
    }

    #[test]
    fn parsing_is_deterministic(seed in proptest::collection::vec(any::<u8>(), 0..256)) {
        // For a fixed, valid document the parse result is stable across calls.
        let _ = seed;
        let a: Sbml = parse_sbml(VALID_L2).expect("valid doc parses");
        let b: Sbml = parse_sbml(VALID_L2).expect("valid doc parses again");
        prop_assert_eq!(a.clone(), b.clone());
        // Validation is likewise deterministic.
        prop_assert_eq!(validate_sbml(&a).errors, validate_sbml(&b).errors);
    }
}
