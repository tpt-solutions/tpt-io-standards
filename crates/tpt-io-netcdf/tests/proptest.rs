//! Property-based tests (proptest) for NetCDF header invariants.
//!
//! Focus: the header round-trip must be byte-stable across arbitrary valid
//! models, and the parser must never panic on arbitrary bytes.

use proptest::prelude::*;
use tpt_io_netcdf::header::{Attr, Dim, NcType, NetCdf, Var};
use tpt_io_netcdf::parser::parse_netcdf;
use tpt_io_netcdf::writer::{round_trip, serialize_header};

/// Build an arbitrary but valid `NetCdf` model (1-3 dims, optional unlimited
/// first dim, one variable over all dims, one global + one variable attr).
fn arb_netcdf() -> impl Strategy<Value = NetCdf> {
    (1u8..=3, proptest::bool::ANY).prop_flat_map(|(ndim, first_unlim)| {
        proptest::collection::vec(proptest::sample::select(vec![1usize, 2, 3, 4]), ndim as usize)
            .prop_map(move |lens| {
                let mut dims = Vec::new();
                let mut numrecs = 0u32;
                for (i, &l) in lens.iter().enumerate() {
                    let unlimited = i == 0 && first_unlim;
                    if unlimited {
                        numrecs = l as u32;
                    }
                    dims.push(Dim { name: format!("d{i}"), len: l as u32, unlimited });
                }
                let product: usize = lens.iter().product();
                let vlen = (product * 4) as u64; // float32 element size
                let v = Var {
                    name: "v".into(),
                    nc_type: NcType::Float,
                    dim_ids: (0..ndim as u32).collect(),
                    attrs: vec![Attr {
                        name: "units".into(),
                        nc_type: NcType::Char,
                        value: b"m".to_vec(),
                    }],
                    vlen,
                    begin: 0,
                };
                NetCdf {
                    version: 1,
                    numrecs: if numrecs == 0 { None } else { Some(numrecs) },
                    dims,
                    global_attrs: vec![Attr {
                        name: "title".into(),
                        nc_type: NcType::Char,
                        value: b"x".to_vec(),
                    }],
                    vars: vec![v],
                    header_len: 0,
                }
            })
    })
}

proptest! {
    #[test]
    fn header_round_trip_byte_stable(nc in arb_netcdf()) {
        let rt = round_trip(&nc).expect("round trip must succeed for valid model");
        // Re-serializing the round-tripped model yields identical bytes.
        prop_assert_eq!(serialize_header(&rt), serialize_header(&nc));
        // Parsed dims/vars survive the cycle.
        prop_assert_eq!(rt.dims, nc.dims);
        prop_assert_eq!(rt.vars, nc.vars);
    }

    #[test]
    fn generated_file_reparses(nc in arb_netcdf()) {
        let bytes = serialize_header(&nc);
        let parsed = parse_netcdf(&bytes).expect("generated header must parse");
        prop_assert_eq!(parsed.dims, nc.dims);
        prop_assert_eq!(parsed.vars, nc.vars);
        prop_assert_eq!(parsed.global_attrs, nc.global_attrs);
    }

    #[test]
    fn parse_never_panics(data in proptest::collection::vec(any::<u8>(), 0..512)) {
        // The parser must return Ok/Err, never panic, on arbitrary bytes.
        let _ = parse_netcdf(&data);
    }
}
