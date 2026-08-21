//! Integration tests for `tpt-io-netcdf`: header round-trip, property tests,
//! and coordinate-variable detection.

use tpt_io_core::{Reader, Writer};
use tpt_io_netcdf::header::{Attr, Dim, NcType, NetCdf, Var};
use tpt_io_netcdf::{NetCdfReader, NetCdfWriter};

fn sample() -> NetCdf {
    NetCdf {
        version: 1,
        numrecs: Some(1),
        dims: vec![
            Dim { name: "time".into(), len: 1 },
            Dim { name: "lat".into(), len: 3 },
            Dim { name: "lon".into(), len: 4 },
        ],
        global_attrs: vec![Attr {
            name: "title".into(),
            nc_type: NcType::Char,
            value: b"sample".to_vec(),
        }],
        vars: vec![
            Var {
                name: "lat".into(),
                nc_type: NcType::Float,
                dim_ids: vec![1],
                attrs: vec![],
                vlen: 12,
                begin: 0,
            },
            Var {
                name: "temp".into(),
                nc_type: NcType::Double,
                dim_ids: vec![0, 1, 2],
                attrs: vec![Attr {
                    name: "units".into(),
                    nc_type: NcType::Char,
                    value: b"K".to_vec(),
                }],
                vlen: 96,
                begin: 128,
            },
        ],
        header_len: 0,
    }
}

#[test]
fn header_round_trips() {
    let nc = sample();
    let bytes = NetCdfWriter::new().serialize(&nc).unwrap();
    let reparsed = NetCdfReader::new().parse(&bytes).expect("reparse");
    assert_eq!(nc.dims, reparsed.dims);
    assert_eq!(nc.vars, reparsed.vars);
    assert_eq!(nc.global_attrs, reparsed.global_attrs);
    assert_eq!(reparsed.version, 1);
}

#[test]
fn coordinate_variables_detected() {
    let nc = sample();
    let coords: Vec<&str> = nc.coordinate_variables().iter().map(|v| v.name.as_str()).collect();
    assert_eq!(coords, vec!["lat"]);
}

#[test]
fn rejects_unknown_magic() {
    let res = NetCdfReader::new().parse(b"NOTACDF!");
    assert!(res.is_err());
}

#[cfg(test)]
mod proptest_roundtrip {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn roundtrip_is_idempotent(
            version in prop_oneof![Just(1u8), Just(2u8)],
            d0 in "x",
            d1 in "y",
        ) {
            let nc = NetCdf {
                version,
                numrecs: Some(2),
                dims: vec![Dim { name: d0.clone(), len: 2 }, Dim { name: d1.clone(), len: 2 }],
                global_attrs: vec![],
                vars: vec![Var {
                    name: d0,
                    nc_type: NcType::Float,
                    dim_ids: vec![0],
                    attrs: vec![],
                    vlen: 8,
                    begin: 0,
                }],
                header_len: 0,
            };
            let bytes = NetCdfWriter::new().serialize(&nc).unwrap();
            let reparsed = NetCdfReader::new().parse(&bytes).expect("reparse");
            prop_assert_eq!(nc.dims, reparsed.dims);
            prop_assert_eq!(nc.vars, reparsed.vars);
            prop_assert_eq!(nc.version, reparsed.version);
        }

        #[test]
        fn no_panic_on_arbitrary_bytes(data in proptest::collection::vec(any::<u8>(), 0..256)) {
            let _ = NetCdfReader::new().parse(&data);
        }
    }
}
