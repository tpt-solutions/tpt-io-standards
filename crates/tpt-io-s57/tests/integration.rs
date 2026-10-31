//! Integration tests for the S-57 reader.
//!
//! No licensed ENC cells are in the repository, so the files here are built from the rules in
//! ISO 8211 by a small writer below. The writer is test-only and is not part of the crate.

use tpt_io_s57::iso8211::{self, Control, FieldDef, Value};
use tpt_io_s57::{parse, Geometry, Primitive, S57Error};

const FIELD_TERMINATOR: u8 = 0x1E;
const UNIT_TERMINATOR: u8 = 0x1F;
const RECORD_TERMINATOR: u8 = 0x1D;

/// Little-endian bytes of `value`, `width` of them.
fn le(value: u64, width: usize) -> Vec<u8> {
    (0..width).map(|i| (value >> (8 * i)) as u8).collect()
}

/// Builds one ISO 8211 record: a leader, a directory and a field area.
fn record(fields: &[(&str, Vec<u8>)]) -> Vec<u8> {
    let mut directory = Vec::new();
    let mut area = Vec::new();
    for (tag, data) in fields {
        let mut body = data.clone();
        body.push(FIELD_TERMINATOR);
        directory.extend_from_slice(format!("{tag}{:04}{:05}", body.len(), area.len()).as_bytes());
        area.extend_from_slice(&body);
    }
    directory.push(FIELD_TERMINATOR);
    let base = 24 + directory.len();
    let total = base + area.len() + 1;
    // Leader: record length, interchange level, leader id, in-line code, version, application,
    // field control length, field area base, extended character set, then the entry sizes:
    // field length 4, field position 5, reserved 0, field tag 4.
    let mut out = format!("{total:05}3LE1 09{base:05} ! 4504").into_bytes();
    out.extend_from_slice(&directory);
    out.extend_from_slice(&area);
    out.push(RECORD_TERMINATOR);
    out
}

/// A DDR field description: a nine-byte field control, the name, the labels and the formats.
fn ddr_field(name: &str, labels: &str, formats: &str) -> Vec<u8> {
    let mut out = vec![b'1', b'0'];
    out.extend_from_slice(&[b' '; 7]);
    out.extend_from_slice(name.as_bytes());
    out.push(UNIT_TERMINATOR);
    out.extend_from_slice(labels.as_bytes());
    out.push(UNIT_TERMINATOR);
    out.extend_from_slice(formats.as_bytes());
    out
}

fn ddr() -> Vec<u8> {
    record(&[
        ("DSID", ddr_field("Dataset identification", "RCNM!RCID!DSNM!EDTN", "(b11,b14,A,A)")),
        ("DSPM", ddr_field("Dataset parameters", "RCNM!RCID!COMF!SOMF", "(b11,b14,b14,b14)")),
        ("VRID", ddr_field("Vector record", "RCNM!RCID", "(b11,b14)")),
        ("VRPT", ddr_field("Vector pointers", "NAME!ORNT", "(B(40),b11)")),
        ("SG2D", ddr_field("2D coordinates", "YCOO!XCOO", "(b24,b24)")),
        ("SG3D", ddr_field("3D coordinates", "YCOO!XCOO!VE3D", "(b24,b24,b24)")),
        (
            "FRID",
            ddr_field(
                "Feature record",
                "RCNM!RCID!PRIM!GRUP!OBJL!RVER!RUIN",
                "(b11,b14,b11,b11,b12,b12,b11)",
            ),
        ),
        ("ATTF", ddr_field("Feature attributes", "ATTL!ATVL", "(b12,A)")),
        ("FSPT", ddr_field("Spatial pointers", "NAME!ORNT!USAG!MASK", "(B(40),b11,b11,b11)")),
    ])
}

/// A 5-byte record name reference: record name code, then record identifier.
fn name(rcnm: u8, rcid: u32) -> Vec<u8> {
    [vec![rcnm], le(u64::from(rcid), 4)].concat()
}

fn vector(rcnm: u8, rcid: u32, points: &[(i64, i64)], ends: &[(u8, u32)]) -> Vec<u8> {
    let mut fields: Vec<(&str, Vec<u8>)> =
        vec![("VRID", [vec![rcnm], le(u64::from(rcid), 4)].concat())];
    if !ends.is_empty() {
        let mut pointers = Vec::new();
        for &(rcnm, rcid) in ends {
            pointers.extend(name(rcnm, rcid));
            pointers.push(1);
        }
        fields.push(("VRPT", pointers));
    }
    if !points.is_empty() {
        let mut coords = Vec::new();
        for &(lat, lon) in points {
            coords.extend(le(lat as u64, 4));
            coords.extend(le(lon as u64, 4));
        }
        fields.push(("SG2D", coords));
    }
    record(&fields)
}

/// Coordinates in units of 1e-7 degrees, so 50.0 N is 500000000.
fn chart_with(extra: &[Vec<u8>]) -> Vec<u8> {
    let mut file = ddr();
    file.extend(record(&[
        ("DSID", [vec![10], le(1, 4), b"US5TEST\x1f".to_vec(), b"1".to_vec()].concat()),
        ("DSPM", [vec![20], le(1, 4), le(10_000_000, 4), le(10, 4)].concat()),
    ]));
    // Connected nodes 1 and 2 bound edge 5.
    file.extend(vector(120, 1, &[(500_000_000, 0)], &[]));
    file.extend(vector(120, 2, &[(505_000_000, 10_000_000)], &[]));
    file.extend(vector(130, 5, &[(502_500_000, 5_000_000)], &[(120, 1), (120, 2)]));
    for record in extra {
        file.extend_from_slice(record);
    }
    file
}

fn line_feature() -> Vec<u8> {
    let attributes = [le(3, 2), b"ABC\x1f".to_vec(), le(7, 2), b"12.5".to_vec()].concat();
    let references = [name(130, 5), vec![2], vec![1], vec![1]].concat();
    record(&[("FRID", frid(100, 10, 2, 42)), ("ATTF", attributes), ("FSPT", references)])
}

fn frid(rcnm: u8, rcid: u32, prim: u8, objl: u64) -> Vec<u8> {
    [vec![rcnm], le(u64::from(rcid), 4), vec![prim], vec![1], le(objl, 2), le(1, 2), vec![1]]
        .concat()
}

fn point_feature() -> Vec<u8> {
    let references = [name(120, 1), vec![1], vec![1], vec![1]].concat();
    record(&[("FRID", frid(100, 11, 1, 70)), ("FSPT", references)])
}

fn sounding_feature() -> Vec<u8> {
    // 50.0 N, 0.0 E, depth 123 with a sounding multiplier of 10, so 12.3 m.
    let coords = [le(500_000_000, 4), le(0, 4), le(123, 4)].concat();
    record(&[("FRID", frid(100, 12, 1, 150)), ("SG3D", coords)])
}

fn area_without_geometry() -> Vec<u8> {
    record(&[("FRID", frid(100, 13, 3, 300))])
}

fn sample() -> Vec<u8> {
    chart_with(&[line_feature(), point_feature(), sounding_feature(), area_without_geometry()])
}

#[test]
fn reads_dataset_parameters_and_features_in_order() {
    let dataset = parse(&sample()).unwrap();
    assert_eq!(dataset.name, "US5TEST");
    assert_eq!(dataset.edition, "1");
    assert_eq!(dataset.coordinate_multiplier, 10_000_000.0);
    assert_eq!(dataset.sounding_multiplier, 10.0);
    let ids: Vec<u32> = dataset.features.iter().map(|f| f.id).collect();
    assert_eq!(ids, vec![10, 11, 12, 13]);
    assert_eq!(dataset.features[0].object_class, 42);
    assert_eq!(dataset.features[0].primitive, Primitive::Line);
}

#[test]
fn line_geometry_follows_the_edge_and_its_orientation() {
    let dataset = parse(&sample()).unwrap();
    let Geometry::Curves(curves) = &dataset.features[0].geometry else {
        panic!("a line feature should have curves");
    };
    assert_eq!(curves.len(), 1);
    let chain = &curves[0];
    // Edge 5 runs from node 1 to node 2; the reference is reversed, so the chain is 2, 1.
    assert_eq!(chain.len(), 3);
    assert!((chain[0].latitude - 50.5).abs() < 1e-9);
    assert!((chain[0].longitude - 1.0).abs() < 1e-9);
    assert!((chain[1].latitude - 50.25).abs() < 1e-9);
    assert!((chain[1].longitude - 0.5).abs() < 1e-9);
    assert!((chain[2].latitude - 50.0).abs() < 1e-9);
    assert!(chain.iter().all(|p| p.depth.is_none()));
}

#[test]
fn attributes_are_read_with_their_codes_and_text_values() {
    let dataset = parse(&sample()).unwrap();
    let attributes = &dataset.features[0].attributes;
    assert_eq!(attributes.len(), 2);
    assert_eq!(attributes[0].code, 3);
    assert_eq!(attributes[0].value, "ABC");
    assert_eq!(attributes[1].code, 7);
    assert_eq!(attributes[1].value, "12.5");
}

#[test]
fn point_features_resolve_to_their_node_positions() {
    let dataset = parse(&sample()).unwrap();
    let Geometry::Points(points) = &dataset.features[1].geometry else {
        panic!("a point feature should have points");
    };
    assert_eq!(points.len(), 1);
    assert!((points[0].latitude - 50.0).abs() < 1e-9);
    assert_eq!(points[0].depth, None);
}

#[test]
fn soundings_carry_depth_scaled_by_the_sounding_multiplier() {
    let dataset = parse(&sample()).unwrap();
    let Geometry::Points(points) = &dataset.features[2].geometry else {
        panic!("a sounding should have points");
    };
    assert_eq!(points.len(), 1);
    let depth = points[0].depth.unwrap();
    assert!((depth - 12.3).abs() < 1e-9);
}

#[test]
fn feature_without_spatial_reference_has_no_geometry() {
    let dataset = parse(&sample()).unwrap();
    assert_eq!(dataset.features[3].geometry, Geometry::None);
    assert_eq!(dataset.features[3].primitive, Primitive::Area);
}

#[test]
fn feature_referring_to_a_missing_vector_record_is_rejected() {
    let dangling = record(&[
        ("FRID", frid(100, 20, 2, 42)),
        ("FSPT", [name(130, 99), vec![1], vec![1], vec![1]].concat()),
    ]);
    let err = parse(&chart_with(&[dangling])).unwrap_err();
    assert!(matches!(err, S57Error::Invalid(ref what) if what.contains("not in the file")));
}

#[test]
fn file_without_dataset_parameters_is_rejected() {
    let mut file = ddr();
    file.extend(record(&[("FRID", frid(100, 10, 3, 1))]));
    let err = parse(&file).unwrap_err();
    assert!(matches!(err, S57Error::Invalid(ref what) if what.contains("DSPM")));
}

#[test]
fn bytes_that_are_not_iso_8211_are_a_core_error() {
    let err = parse(b"this is not an S-57 file at all").unwrap_err();
    assert!(matches!(err, S57Error::Core(_)));
}

#[test]
fn empty_input_has_no_data_descriptive_record() {
    let err = parse(b"").unwrap_err();
    assert!(matches!(err, S57Error::Invalid(_)));
}

#[test]
fn truncated_files_fail_without_panicking() {
    let bytes = sample();
    for end in 0..bytes.len() {
        let _ = parse(&bytes[..end]);
    }
}

#[test]
fn ddr_describes_each_field_with_its_labels_and_controls() {
    let records = iso8211::read_records(&ddr()).unwrap();
    let dictionary = iso8211::dictionary(&records[0]).unwrap();
    let dsid = &dictionary["DSID"];
    assert_eq!(dsid.labels, vec!["RCNM", "RCID", "DSNM", "EDTN"]);
    assert_eq!(
        dsid.controls,
        vec![
            Control::Binary { width: 1, signed: false },
            Control::Binary { width: 4, signed: false },
            Control::Text(None),
            Control::Text(None),
        ]
    );
    assert_eq!(dictionary["VRPT"].controls[0], Control::Bits(40));
    assert_eq!(dictionary["SG2D"].controls[0], Control::Binary { width: 4, signed: true });
}

#[test]
fn unsupported_format_control_is_rejected() {
    let bad = record(&[("DSID", ddr_field("Dataset identification", "RCNM", "(X)"))]);
    assert!(iso8211::dictionary(&iso8211::read_records(&bad).unwrap()[0]).is_err());
}

#[test]
fn signed_binary_subfields_sign_extend() {
    let def = FieldDef {
        tag: String::from("TEST"),
        labels: vec![String::from("VALUE")],
        controls: vec![Control::Binary { width: 2, signed: true }],
    };
    let subfields = iso8211::decode(&def, &[0xFE, 0xFF]).unwrap();
    assert_eq!(subfields[0].value, Value::Signed(-2));
}

#[test]
fn bit_strings_stay_as_bytes() {
    let def = FieldDef {
        tag: String::from("TEST"),
        labels: vec![String::from("NAME")],
        controls: vec![Control::Bits(40)],
    };
    let subfields = iso8211::decode(&def, &[1, 2, 3, 4, 5]).unwrap();
    assert_eq!(subfields[0].value, Value::Bytes(vec![1, 2, 3, 4, 5]));
}
