//! Integration tests for the MDF 4 reader, against files written by asammdf (see `gen_mdf.py`).

use tpt_io_mdf::{channel, parse, MdfError};

fn sample(name: &str) -> Vec<u8> {
    std::fs::read(format!("{}/tests/samples/{name}", env!("CARGO_MANIFEST_DIR"))).unwrap()
}

/// The `speed` and `count` values as asammdf wrote them, from `expected.txt`.
fn expected() -> (Vec<f64>, Vec<f64>) {
    let text = std::fs::read_to_string(format!(
        "{}/tests/samples/expected.txt",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap();
    let mut speed = Vec::new();
    let mut count = Vec::new();
    for line in text.lines() {
        let mut parts = line.split_whitespace();
        match parts.next() {
            Some("speed") => speed = parts.map(|v| v.parse().unwrap()).collect(),
            Some("count") => count = parts.map(|v| v.parse::<f64>().unwrap()).collect(),
            _ => {}
        }
    }
    (speed, count)
}

#[test]
fn reads_names_units_and_values_of_an_uncompressed_file() {
    let mdf = parse(&sample("plain.mf4")).unwrap();
    assert_eq!(mdf.version, "4.10");
    assert_eq!(mdf.groups.len(), 1);
    let group = &mdf.groups[0];
    assert_eq!(group.records, 100);
    let names: Vec<&str> = group.channels.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(names, ["time", "speed", "count"]);

    let (speed, count) = expected();
    let speed_ch = channel(&mdf, "speed").unwrap();
    assert_eq!(speed_ch.unit.as_deref(), Some("km/h"));
    assert_eq!(speed_ch.values, speed);
    assert_eq!(channel(&mdf, "count").unwrap().values, count);

    let time = channel(&mdf, "time").unwrap();
    assert_eq!(time.values.len(), 100);
    for (i, v) in time.values.iter().enumerate() {
        assert_eq!(*v, i as f64 * 0.01);
    }
}

#[test]
fn deflate_compressed_blocks_give_the_same_channels() {
    let plain = parse(&sample("plain.mf4")).unwrap();
    let packed = parse(&sample("compressed.mf4")).unwrap();
    assert_eq!(plain.groups, packed.groups);
}

#[test]
fn non_mdf_input_is_rejected() {
    assert!(matches!(parse(b"not an MDF file at all, just text"), Err(MdfError::Invalid(_))));
}

#[test]
fn truncated_files_are_errors_not_panics() {
    let full = sample("plain.mf4");
    for cut in [70, 200, 1000, full.len() - 1] {
        assert!(parse(&full[..cut]).is_err(), "cut at {cut} should fail");
    }
}
