//! Integration tests for the TLE reader and writer.

use tpt_io_tle::{checksum, parse, write, Tle, TleError};

const ISS: &[u8] = b"ISS (ZARYA)
1 25544U 98067A   24001.50000000  .00016717  00000+0  10270-3 0  9008
2 25544  51.6400 208.9163 0006317  69.9862  25.2906 15.49560000 10009
NOAA 19
1 33591U 09005A   24001.20000000  .00000012  00000-0  12345-4 0  9996
2 33591  99.1900  54.2000 0014000 100.0000 260.1000 14.12500000 55550
";

#[test]
fn checksum_matches_the_printed_digit() {
    let line1 = "1 25544U 98067A   24001.50000000  .00016717  00000+0  10270-3 0  9008";
    assert_eq!(checksum(line1), 8);
    assert!(parse(ISS).is_ok());
}

#[test]
fn reads_all_sets_with_names_and_fields() {
    let sets = parse(ISS).unwrap();
    assert_eq!(sets.len(), 2);
    let iss = &sets[0];
    assert_eq!(iss.name.as_deref(), Some("ISS (ZARYA)"));
    assert_eq!(iss.satellite_number, 25544);
    assert_eq!(iss.international_designator, "98067A");
    assert_eq!(iss.epoch_year, 2024);
    assert!((iss.epoch_day - 1.5).abs() < 1e-12);
    assert!((iss.mean_motion_dot - 0.00016717).abs() < 1e-12);
    assert_eq!(iss.mean_motion_ddot, 0.0);
    assert!((iss.bstar - 1.0270e-4).abs() < 1e-12);
    assert!((iss.inclination - 51.64).abs() < 1e-9);
    assert!((iss.eccentricity - 0.0006317).abs() < 1e-12);
    assert!((iss.mean_motion - 15.4956).abs() < 1e-9);
    assert_eq!(iss.revolution_number, 1000);
    assert_eq!(iss.element_set_number, 900);
    assert_eq!(sets[1].name.as_deref(), Some("NOAA 19"));
    assert!((sets[1].bstar - 1.2345e-5).abs() < 1e-14);
}

#[test]
fn write_then_parse_is_identity() {
    let sets = parse(ISS).unwrap();
    let text = write(&sets);
    assert_eq!(parse(&text).unwrap(), sets);
}

#[test]
fn written_lines_have_canonical_width_and_checksums() {
    let text = write(&parse(ISS).unwrap());
    let lines: Vec<&str> = std::str::from_utf8(&text).unwrap().lines().collect();
    assert_eq!(lines[1].len(), 69);
    assert_eq!(lines[2].len(), 69);
    assert_eq!(checksum(lines[1]).to_string(), &lines[1][68..]);
    assert_eq!(checksum(lines[2]).to_string(), &lines[2][68..]);
}

#[test]
fn a_bad_checksum_is_reported_with_the_satellite() {
    let bad = b"1 25544U 98067A   24001.50000000  .00016717  00000+0  10270-3 0  9009
2 25544  51.6400 208.9163 0006317  69.9862  25.2906 15.49560000 10009
";
    assert_eq!(parse(bad), Err(TleError::Checksum { satellite: 25544, line: 1 }));
}

#[test]
fn mismatched_and_short_lines_are_errors() {
    let short = b"1 25544U 98067A\n2 25544\n";
    assert!(parse(short).is_err());
    let lonely = b"1 25544U 98067A   24001.50000000  .00016717  00000+0  10270-3 0  9000\n";
    assert!(parse(lonely).is_err());
}

#[test]
fn generated_sets_round_trip() {
    let mut x: u64 = 0x2545_F491_4F6C_DD1D;
    let mut next = move || {
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        x
    };
    // Every value is an integer divided by the power of ten the TLE column carries, so the
    // canonical text reads back to the same f64.
    for _ in 0..200 {
        let t = Tle {
            name: Some(alloc_name(next())),
            satellite_number: (next() % 99_999) as u32 + 1,
            classification: 'U',
            international_designator: "21001A".into(),
            epoch_year: 2000 + (next() % 26) as u16,
            epoch_day: (100_000_000 + next() % 36_000_000) as f64 / 1e8,
            mean_motion_dot: (next() % 100_000) as f64 / 1e8,
            mean_motion_ddot: 0.0,
            bstar: 0.0,
            ephemeris_type: 0,
            element_set_number: (next() % 999) as u32,
            inclination: (next() % 1_800_000) as f64 / 1e4,
            raan: (next() % 3_600_000) as f64 / 1e4,
            eccentricity: (next() % 9_999_999) as f64 / 1e7,
            argument_of_perigee: (next() % 3_600_000) as f64 / 1e4,
            mean_anomaly: (next() % 3_600_000) as f64 / 1e4,
            mean_motion: (100_000_000 + next() % 1_500_000_000) as f64 / 1e8,
            revolution_number: (next() % 99_999) as u32,
        };
        let text = write(std::slice::from_ref(&t));
        let back = parse(&text).unwrap_or_else(|e| panic!("{e}\n{}", String::from_utf8_lossy(&text)));
        assert_eq!(back.len(), 1);
        assert_eq!(back[0], t);
    }
}

fn alloc_name(v: u64) -> String {
    format!("SAT {:04}", v % 10_000)
}

#[test]
fn arbitrary_bytes_never_panic() {
    let mut x: u64 = 5;
    for _ in 0..500 {
        let len = (x % 220) as usize;
        let bytes: Vec<u8> = (0..len)
            .map(|_| {
                x ^= x << 13;
                x ^= x >> 7;
                x ^= x << 17;
                b"12 -.+E\n0123456789ABCabc"[(x % 24) as usize]
            })
            .collect();
        let _ = parse(&bytes);
    }
}

#[test]
fn struct_is_clonable_and_debuggable() {
    let t = parse(ISS).unwrap().remove(0);
    let _ = format!("{:?}", t.clone());
}
