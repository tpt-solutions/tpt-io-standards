//! Golden-file test: parse a checked-in canonical property-package document and
//! assert the structure, then confirm it round-trips back to identical text.

use tpt_io_capeopen::model::{BaseProperty, Phase};
use tpt_io_capeopen::parser::parse_property_package;
use tpt_io_capeopen::writer::encode_property_package;

const SAMPLE: &str = include_str!("samples/binary_distillation.pp");

#[test]
fn golden_file_parses() {
    let pp = parse_property_package(SAMPLE.as_bytes()).expect("golden sample must parse");
    pp.validate().expect("golden sample must be a valid package");

    assert_eq!(pp.name, "PengRobinson");
    assert_eq!(pp.components.len(), 3);
    assert_eq!(pp.components.get("METHANOL").unwrap().cas_number.as_deref(), Some("67-56-1"));
    assert_eq!(pp.supported_phases, vec![Phase::Vapor, Phase::Liquid]);
    assert_eq!(
        pp.supported_single_phase,
        vec![
            BaseProperty::Temperature,
            BaseProperty::Pressure,
            BaseProperty::Enthalpy,
            BaseProperty::MolarVolume,
            BaseProperty::Viscosity,
        ]
    );
    assert_eq!(
        pp.supported_two_phase,
        vec![BaseProperty::VaporFraction, BaseProperty::EquilibriumTemperature]
    );
}

#[test]
fn golden_file_round_trips_to_identical_text() {
    let pp = parse_property_package(SAMPLE.as_bytes()).unwrap();
    // Re-encoding yields the canonical form, which reparses to the same model and
    // (because the golden file is already canonical) the same bytes.
    let reencoded = encode_property_package(&pp);
    assert_eq!(SAMPLE, reencoded);
}
