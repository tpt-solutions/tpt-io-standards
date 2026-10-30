//! Property-based tests (proptest) for parser invariants and crash-freedom.

use proptest::prelude::*;
use tpt_io_core::Writer;
use tpt_io_welllog::binary;
use tpt_io_welllog::model::{Curve, WellLog};
use tpt_io_welllog::parser::{parse_str, LasReader};

/// Build a `WellLog` with a default `WellInfo` so that serialize → parse is
/// exactly identity (the writer only emits curves, data, and a minimal header).
fn build_log(ncols: usize, nrows: usize, vals: &[f64]) -> WellLog {
    let mut curves = Vec::new();
    for i in 0..ncols {
        curves.push(Curve {
            mnemonic: format!("C{i}"),
            unit: "M".to_string(),
            api_code: "0".to_string(),
            description: "d".to_string(),
        });
    }
    let mut data = Vec::with_capacity(nrows);
    for r in 0..nrows {
        let mut row = Vec::with_capacity(ncols);
        for c in 0..ncols {
            row.push(vals[r * ncols + c]);
        }
        data.push(row);
    }
    WellLog {
        version: 2.0,
        wrap: false,
        well: Default::default(),
        parameters: Vec::new(),
        curves,
        data,
    }
}

proptest! {
    #[test]
    fn las_round_trip_is_identity(
        (ncols, nrows, vals) in (1usize..6, 0usize..30).prop_flat_map(|(ncols, nrows)| {
            proptest::collection::vec(-1000.0..1000.0f64, ncols * nrows)
                .prop_map(move |vals| (ncols, nrows, vals))
        }),
    ) {
        let log = build_log(ncols, nrows, &vals);
        let bytes = LasReader::new().serialize(&log).unwrap();
        let reparsed = parse_str(core::str::from_utf8(&bytes).unwrap()).unwrap();
        prop_assert_eq!(log, reparsed);
    }

    #[test]
    fn parse_str_never_panics(s in ".*") {
        // Arbitrary text must yield Ok or Err, never panic.
        let _ = parse_str(&s);
    }

    #[test]
    fn binary_parse_never_panics(bytes in proptest::collection::vec(any::<u8>(), 0..512)) {
        // Arbitrary bytes must yield Ok or Err, never panic.
        let _ = binary::parse(&bytes);
    }
}
