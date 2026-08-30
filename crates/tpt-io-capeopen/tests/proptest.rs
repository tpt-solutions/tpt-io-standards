//! Property-based tests (proptest) for CAPE-OPEN property-package invariants.
//!
//! We generate random but well-formed packages (safe component-id charset, known
//! phase/property vocabularies) and assert that the canonical text codec is
//! stable: encode → parse → encode yields byte-identical output, and the parsed
//! model equals the original.

use proptest::prelude::*;

use tpt_io_capeopen::model::{BaseProperty, ComponentList, Phase, PropertyPackage};
use tpt_io_capeopen::parser::parse_property_package;
use tpt_io_capeopen::writer::encode_property_package;

fn phase_strat() -> impl Strategy<Value = Phase> {
    prop_oneof![
        Just(Phase::Vapor),
        Just(Phase::Liquid),
        Just(Phase::Liquid1),
        Just(Phase::Solid),
        Just(Phase::Overall),
    ]
}

fn prop_strat() -> impl Strategy<Value = BaseProperty> {
    prop_oneof![
        Just(BaseProperty::Temperature),
        Just(BaseProperty::Pressure),
        Just(BaseProperty::Enthalpy),
        Just(BaseProperty::Viscosity),
        Just(BaseProperty::VaporFraction),
        Just(BaseProperty::Kvalue),
    ]
}

proptest! {
    #[test]
    fn round_trip_stable(
        name in "[A-Za-z0-9_]{1,12}",
        ids in proptest::collection::vec("[A-Za-z0-9_]{1,12}", 1..6),
        phases in proptest::collection::vec(phase_strat(), 1..4),
        single in proptest::collection::vec(prop_strat(), 0..5),
        two in proptest::collection::vec(prop_strat(), 0..4),
    ) {
        let id_refs: Vec<&str> = ids.iter().map(|s| s.as_str()).collect();
        let mut pp = PropertyPackage::new(name, ComponentList::from_ids(&id_refs));
        pp.supported_phases = phases;
        pp.supported_single_phase = single;
        pp.supported_two_phase = two;

        let text = encode_property_package(&pp);
        let back = parse_property_package(text.as_bytes()).expect("parse");
        let text2 = encode_property_package(&back);
        prop_assert_eq!(text, text2, "canonical encode must be stable");
        prop_assert_eq!(pp, back, "parsed model must equal original");
    }

    #[test]
    fn component_list_accepts_distinct_ids(
        ids in proptest::collection::vec("[A-Za-z0-9_]{1,12}", 1..6),
    ) {
        let id_refs: Vec<&str> = ids.iter().map(|s| s.as_str()).collect();
        let list = ComponentList::from_ids(&id_refs);
        // Parse of the encoded form preserves every id.
        let text = encode_property_package(&PropertyPackage::new("P", list.clone()));
        let back = parse_property_package(text.as_bytes()).unwrap();
        prop_assert_eq!(list, back.components);
    }
}
