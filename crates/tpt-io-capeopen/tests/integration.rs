//! Integration tests for `tpt-io-capeopen`: property-package round-trip through
//! the canonical text codec, flowsheet connectivity validation, and the
//! feature-gated downstream integration stubs.

use tpt_io_capeopen::flowsheet::Flowsheet;
use tpt_io_capeopen::model::{
    BaseProperty, CalculationKind, ComponentList, Phase, PropertyCalculationRequest,
    PropertyPackage, StreamState,
};
use tpt_io_capeopen::parser::{parse_property_package, PropertyPackageReader};
use tpt_io_capeopen::ports::{Port, PortDirection};
use tpt_io_capeopen::writer::{encode_property_package, serialize};
use tpt_io_core::Reader;

fn sample_package() -> PropertyPackage {
    let mut pp = PropertyPackage::new(
        "PengRobinson",
        ComponentList::from_ids(&["WATER", "METHANOL", "ETHANOL"]),
    );
    pp.supported_phases = vec![Phase::Vapor, Phase::Liquid];
    pp.supported_single_phase = vec![
        BaseProperty::Temperature,
        BaseProperty::Pressure,
        BaseProperty::Enthalpy,
        BaseProperty::MolarVolume,
        BaseProperty::Viscosity,
    ];
    pp.supported_two_phase =
        vec![BaseProperty::VaporFraction, BaseProperty::EquilibriumTemperature];
    pp
}

#[test]
fn package_round_trips_through_text_codec() {
    let pp = sample_package();
    let text = encode_property_package(&pp);
    let back = parse_property_package(text.as_bytes()).expect("parse");
    assert_eq!(pp, back);

    // And through the core traits.
    let bytes = serialize(&pp).unwrap();
    let via_traits: PropertyPackage = PropertyPackageReader::new().parse(&bytes).unwrap();
    assert_eq!(pp, via_traits);
}

#[test]
fn reader_rejects_invalid_package() {
    let mut pp = sample_package();
    pp.components = ComponentList::new(); // empty -> invalid
    let bytes = serialize(&pp).unwrap();
    assert!(PropertyPackageReader::new().parse(&bytes).is_err());
}

#[test]
fn build_single_phase_request() {
    let pp = sample_package();
    let req = PropertyCalculationRequest::new(
        CalculationKind::SinglePhase,
        vec![Phase::Vapor],
        pp.supported_single_phase.clone(),
    );
    assert_eq!(req.kind, CalculationKind::SinglePhase);
    assert_eq!(req.phases, vec![Phase::Vapor]);
    assert_eq!(req.properties.len(), 5);
}

#[test]
fn stream_state_validates_against_components() {
    let comps = ComponentList::from_ids(&["A", "B"]);
    let mut st = StreamState::new(2);
    st.overall_mole_fractions = vec![0.3, 0.7];
    assert!(st.validate(&comps, 1e-9).is_ok());
    st.overall_mole_fractions = vec![1.0, 0.0];
    assert!(st.validate(&comps, 1e-9).is_ok());
}

#[test]
fn flowsheet_valid_linear_topology() {
    let mut fs = Flowsheet::new();
    let mut mixer = tpt_io_capeopen::ports::UnitOperation::new("Mixer");
    mixer.add_port(Port::new("IN1", PortDirection::Inlet));
    mixer.add_port(Port::new("IN2", PortDirection::Inlet));
    mixer.add_port(Port::new("OUT", PortDirection::Outlet));
    fs.add_unit_operation(mixer);
    fs.add_unit_operation({
        let mut s = tpt_io_capeopen::ports::UnitOperation::new("Column");
        s.add_port(Port::new("FEED", PortDirection::Inlet));
        s.add_port(Port::new("DIST", PortDirection::Outlet));
        s
    });

    fs.connect("Mixer", "OUT", "FEED").unwrap();
    fs.connect("Column", "FEED", "FEED").unwrap();
    fs.connect("Mixer", "IN1", "S1").unwrap();
    fs.connect("Mixer", "IN2", "S2").unwrap();
    assert!(fs.validate().is_ok());
    assert_eq!(fs.streams.len(), 3);
}

#[test]
fn flowsheet_rejects_dangling_inlet() {
    let mut fs = Flowsheet::new();
    let mut uo = tpt_io_capeopen::ports::UnitOperation::new("Pump");
    uo.add_port(Port::new("IN", PortDirection::Inlet));
    uo.add_port(Port::new("OUT", PortDirection::Outlet));
    fs.add_unit_operation(uo);
    assert!(fs.validate().is_err());
}

#[test]
fn flowsheet_rejects_one_stream_two_inlets() {
    let mut fs = Flowsheet::new();
    let mut uo = tpt_io_capeopen::ports::UnitOperation::new("Split");
    uo.add_port(Port::new("IN", PortDirection::Inlet));
    uo.add_port(Port::new("O1", PortDirection::Outlet));
    uo.add_port(Port::new("O2", PortDirection::Outlet));
    fs.add_unit_operation(uo);
    fs.connect("Split", "IN", "S").unwrap();
    fs.connect("Split", "O1", "S").unwrap();
    fs.connect("Split", "O2", "S").unwrap();
    assert!(fs.validate().is_err());
}

#[cfg(feature = "thermo-integration")]
#[test]
fn thermo_integration_stub() {
    use tpt_io_capeopen::integration::thermo::to_property_model;
    let m = to_property_model(&sample_package()).expect("valid package");
    assert_eq!(m.n_components, 3);
    assert_eq!(m.n_properties, 7);
}

#[cfg(feature = "eng-unit-ops-integration")]
#[test]
fn unit_ops_integration_stub() {
    use tpt_io_capeopen::integration::unit_ops::to_process_model;
    let mut fs = Flowsheet::new();
    let mut uo = tpt_io_capeopen::ports::UnitOperation::new("X");
    uo.add_port(Port::new("IN", PortDirection::Inlet));
    uo.add_port(Port::new("OUT", PortDirection::Outlet));
    fs.add_unit_operation(uo);
    let m = to_process_model(&fs);
    assert_eq!(m.units, 1);
    assert!(!m.connected);
}
