//! Integration tests for the MATLAB v7.3 reader, against `tests/samples/mat73_sample.mat` (see `gen_mat73.py`).

use tpt_io_hdf5::matlab::{is_mat73, parse_mat73};
use tpt_io_hdf5::{DecodedData, Hdf5Error};

fn sample() -> Vec<u8> {
    std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/samples/mat73_sample.mat")).unwrap()
}

#[test]
fn detects_the_header_and_keeps_its_text() {
    let d = sample();
    assert!(is_mat73(&d));
    let m = parse_mat73(&d).unwrap();
    assert!(m.description.starts_with("MATLAB 7.3 MAT-file"));
}

#[test]
fn reads_all_array_variables_and_skips_groups() {
    let m = parse_mat73(&sample()).unwrap();
    let mut names: Vec<&str> = m.variables.iter().map(|v| v.name.as_str()).collect();
    names.sort_unstable();
    assert_eq!(names, vec!["a", "name", "z"]);
}

#[test]
fn double_matrix_has_matlab_dimensions_and_values() {
    let m = parse_mat73(&sample()).unwrap();
    let a = m.variables.iter().find(|v| v.name == "a").unwrap();
    assert_eq!(a.class, "double");
    assert_eq!(a.dims, vec![2, 3]);
    assert_eq!(a.as_f64().unwrap(), vec![1.0, 4.0, 2.0, 5.0, 3.0, 6.0]);
}

#[test]
fn compressed_double_vector_is_inflated() {
    let m = parse_mat73(&sample()).unwrap();
    let z = m.variables.iter().find(|v| v.name == "z").unwrap();
    assert_eq!(z.dims, vec![4, 1]);
    assert_eq!(z.as_f64().unwrap(), vec![0.5, 1.5, 2.5, 3.5]);
}

#[test]
fn char_variable_is_returned_as_raw_uint16_bytes() {
    let m = parse_mat73(&sample()).unwrap();
    let c = m.variables.iter().find(|v| v.name == "name").unwrap();
    assert_eq!(c.class, "char");
    assert_eq!(c.data, DecodedData::Raw(vec![104, 0, 105, 0]));
    assert!(c.as_f64().is_none());
}

#[test]
fn non_mat_input_and_truncated_header_are_errors() {
    let hdf5 =
        std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/samples/example4.h5")).unwrap();
    assert!(matches!(parse_mat73(&hdf5), Err(Hdf5Error::Unsupported(_))));
    let short = &sample()[..512];
    assert!(parse_mat73(short).is_err());
}
