//! Property-based tests (proptest) for NIfTI header invariants.

use proptest::prelude::*;
use tpt_io_nifti::header::{DataType, Nifti1Header};
use tpt_io_nifti::writer::{round_trip, serialize_header};

proptest! {
    #[test]
    fn header_round_trip_invariant(
        ndim in 1u8..4,
        d0 in 1u16..5,
        d1 in 1u16..5,
        d2 in 1u16..5,
        code in prop::sample::select(vec![2i16, 4, 8, 16, 64, 256, 512, 1024]),
    ) {
        let mut h = Nifti1Header {
            raw: vec![],
            version: 1,
            ndim,
            dim: [d0, d1, d2, 1, 1, 1, 1],
            pixdim: [1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0],
            datatype: DataType::from_code(code),
            bitpix: DataType::from_code(code).bitpix(),
            vox_offset: 352,
            scl_slope: 1.0,
            scl_inter: 0.0,
            sform_code: 1,
            qform_code: 0,
            quatern: [0.0; 3],
            qoffset: [0.0; 3],
            srow: [[0.0; 4]; 3],
            descrip: String::new(),
            xyzt_units: 0,
        };
        h.srow[0][0] = 2.0;
        h.srow[1][1] = 2.0;
        h.srow[2][2] = 2.0;
        let rt = round_trip(&h).expect("round trip");
        prop_assert_eq!(rt.ndim, ndim);
        prop_assert_eq!(rt.datatype, h.datatype);
        prop_assert_eq!(serialize_header(&rt), serialize_header(&h));
    }

    #[test]
    fn voxel_count_invariant(
        d0 in 1u16..4,
        d1 in 1u16..4,
        d2 in 1u16..4,
    ) {
        let h = Nifti1Header {
            raw: vec![],
            version: 1,
            ndim: 3,
            dim: [d0, d1, d2, 1, 1, 1, 1],
            pixdim: [1.0; 7],
            datatype: DataType::Float32,
            bitpix: 32,
            vox_offset: 352,
            scl_slope: 1.0,
            scl_inter: 0.0,
            sform_code: 0,
            qform_code: 0,
            quatern: [0.0; 3],
            qoffset: [0.0; 3],
            srow: [[0.0; 4]; 3],
            descrip: String::new(),
            xyzt_units: 0,
        };
        prop_assert_eq!(h.voxel_count(), d0 as usize * d1 as usize * d2 as usize);
    }
}
