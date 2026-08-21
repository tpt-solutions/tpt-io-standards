//! Integration tests for NIfTI parsing and coordinate transforms.

use tpt_io_core::{Reader, Validator};
use tpt_io_nifti::coord::{ras_to_lps, voxel_to_world, world_to_voxel};
use tpt_io_nifti::header::{DataType, Nifti1Header};
use tpt_io_nifti::model::NiftiFile;
use tpt_io_nifti::parser::{NiftiReader, NiftiValidator};
use tpt_io_nifti::writer::{round_trip, serialize_header};

/// Construct a valid NIfTI-1 `.nii` byte stream with a tiny float32 volume.
fn make_nii() -> Vec<u8> {
    let mut raw = vec![0u8; 348];
    raw[0..4].copy_from_slice(&348i32.to_le_bytes());
    raw[40..42].copy_from_slice(&3i16.to_le_bytes()); // ndim = 3
    for (i, d) in [2u16, 2, 2].iter().enumerate() {
        raw[42 + i * 2..44 + i * 2].copy_from_slice(&(*d as i16).to_le_bytes());
    }
    raw[70..72].copy_from_slice(&16i16.to_le_bytes()); // float32
    raw[72..74].copy_from_slice(&32i16.to_le_bytes()); // bitpix
    raw[108..112].copy_from_slice(&352f32.to_le_bytes()); // vox_offset (348 + 4 ext)
    for i in 1..4 {
        raw[76 + i * 4..80 + i * 4].copy_from_slice(&1.0f32.to_le_bytes());
    }
    // sform affine: identity with 2.0 spacing.
    raw[252..254].copy_from_slice(&1i16.to_le_bytes()); // sform_code = 1
    for r in 0..3 {
        let base = 280 + r * 16;
        let mut val = [0.0f32, 0.0, 0.0, (r as f32) * 10.0];
        val[r] = 2.0; // 2.0 spacing on the diagonal
        for c in 0..4 {
            raw[base + c * 4..base + c * 4 + 4].copy_from_slice(&val[c].to_le_bytes());
        }
    }
    raw[344..348].copy_from_slice(b"n+1\0");

    let mut out = raw;
    out.extend_from_slice(&[0u8; 4]); // 4-byte extension padding
                                      // 8 float32 voxels.
    for v in 0..8u8 {
        out.extend_from_slice(&(v as f32).to_le_bytes());
    }
    out
}

#[test]
fn parse_single_file() {
    let bytes = make_nii();
    let reader = NiftiReader::new();
    let f: NiftiFile = reader.parse(&bytes).expect("parse");
    assert_eq!(f.header.ndim, 3);
    assert_eq!(f.header.voxel_count(), 8);
    assert_eq!(f.header.datatype, DataType::Float32);
    assert_eq!(f.data.len(), 32);
    assert_eq!(f.voxel(0), Some(0.0));
    assert_eq!(f.voxel(7), Some(7.0));
    assert_eq!(f.voxel(100), None);
}

#[test]
fn validator_ok() {
    let bytes = make_nii();
    let f: NiftiFile = NiftiReader::new().parse(&bytes).unwrap();
    NiftiValidator::new().validate_syntactic(&f).unwrap();
    NiftiValidator::new().validate_semantic(&f).unwrap();
}

#[test]
fn affine_and_coords() {
    let bytes = make_nii();
    let f: NiftiFile = NiftiReader::new().parse(&bytes).unwrap();
    let aff = f.header.affine();
    // Identity-ish with spacing 2 and origin (0,10,20).
    let w = voxel_to_world(&aff, 1.0, 1.0, 1.0);
    assert!((w[0] - 2.0).abs() < 1e-5);
    assert!((w[1] - 12.0).abs() < 1e-5);
    assert!((w[2] - 22.0).abs() < 1e-5);
    // Round-trip back to voxel.
    let v = world_to_voxel(&aff, w[0], w[1], w[2]).unwrap();
    assert!((v[0] - 1.0).abs() < 1e-4);
    // RAS -> LPS flips first two axes.
    let lps = ras_to_lps(w);
    assert_eq!(lps, [-w[0], -w[1], w[2]]);
}

#[test]
fn header_round_trip() {
    let bytes = make_nii();
    let f: NiftiFile = NiftiReader::new().parse(&bytes).unwrap();
    let h: Nifti1Header = f.header.clone();
    let rt = round_trip(&h).expect("round trip");
    assert_eq!(rt.ndim, h.ndim);
    assert_eq!(rt.dim, h.dim);
    assert_eq!(rt.datatype, h.datatype);
    assert_eq!(rt.vox_offset, h.vox_offset);
    // Re-serializing yields identical bytes.
    assert_eq!(serialize_header(&rt), serialize_header(&h));
}

#[test]
fn hdr_img_dual_file() {
    let bytes = make_nii();
    let mut hdr = bytes[..348].to_vec();
    // In a dual-file layout the .img starts at its own offset 0.
    hdr[108..112].copy_from_slice(&0f32.to_le_bytes());
    let vox_offset = 352usize;
    let img = bytes[vox_offset..].to_vec();
    let f = tpt_io_nifti::parser::parse_hdr_img(&hdr, &img).expect("dual file");
    assert_eq!(f.voxel_count(), 8);
    assert_eq!(f.voxel(3), Some(3.0));
}
