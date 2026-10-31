//! Integration tests for the RINEX 3 observation and SP3 readers.

use tpt_io_gnss::{parse_rinex_obs, parse_sp3, GnssError};

const RINEX: &[u8] = b"     3.04           OBSERVATION DATA    M                   RINEX VERSION / TYPE
STATION1                                                    MARKER NAME
1111111.1111  2222222.2222  3333333.3333                    APPROX POSITION XYZ
G    3 C1C L1C S1C                                            SYS / # / OBS TYPES
E    2 C1X L1X                                                SYS / # / OBS TYPES
                                                              END OF HEADER
> 2024 01 01 00 00  0.0000000  0  2
G01  20000000.123        1234.567 8
E05  21000000.500  
> 2024 01 01 00 00 30.0000000  0  1
G01  20000100.000        1234.600 8
";

#[test]
fn reads_header_and_epochs() {
    let obs = parse_rinex_obs(RINEX).unwrap();
    assert_eq!(obs.version, "3.04");
    assert_eq!(obs.marker.as_deref(), Some("STATION1"));
    assert_eq!(obs.approx_position, Some([1111111.1111, 2222222.2222, 3333333.3333]));
    assert_eq!(obs.obs_types.len(), 2);
    assert_eq!(obs.obs_types[0].1, ["C1C", "L1C", "S1C"]);
    assert_eq!(obs.epochs.len(), 2);
    assert_eq!(obs.epochs[1].time.5, 30.0);
}

#[test]
fn satellites_carry_values_with_missing_entries() {
    let obs = parse_rinex_obs(RINEX).unwrap();
    let g01 = &obs.epochs[0].satellites[0];
    assert_eq!(g01.id, "G01");
    assert_eq!(g01.values[0], Some(20000000.123));
    assert_eq!(g01.values[1], Some(1234.567));
    assert_eq!(g01.values[2], None);
    let e05 = &obs.epochs[0].satellites[1];
    assert_eq!(e05.values, vec![Some(21000000.5), None]);
}

#[test]
fn rinex_without_end_of_header_is_an_error() {
    assert!(matches!(parse_rinex_obs(b"     3.04           OBSERVATION DATA\n"), Err(GnssError::Invalid(_))));
    assert!(parse_rinex_obs(b"     3.04\n                  END OF HEADER\nG01 1.0\n").is_err());
}

const SP3: &[u8] = b"#dP2024 01 01 00 00 00.00000000   2 ORBIT IGS20 FIT  IGS
*  2024 01 01 00 00 00.00000000
PG01  -5000.123456   8000.000000  12000.500000    -123.456789
PG02  100.000000  200.000000  300.000000  999999.999999
*  2024 01 01 00 15 00.00000000
PG01  -5001.000000   8001.000000  12001.000000    -123.500000
";

#[test]
fn reads_sp3_epochs_positions_and_clock() {
    let sp3 = parse_sp3(SP3).unwrap();
    assert_eq!(sp3.version, 'd');
    assert_eq!(sp3.epochs.len(), 2);
    assert_eq!(sp3.epochs[0].time, "2024 01 01 00 00 00.00000000");
    let g01 = &sp3.epochs[0].positions[0];
    assert_eq!(g01.id, "G01");
    assert_eq!(g01.position_km, [-5000.123456, 8000.0, 12000.5]);
    assert_eq!(g01.clock_us, Some(-123.456789));
    assert_eq!(sp3.epochs[0].positions[1].clock_us, None);
}

#[test]
fn sp3_without_a_version_line_is_an_error() {
    assert!(matches!(parse_sp3(b"*  2024 01 01 00 00 0.0\n"), Err(GnssError::Invalid(_))));
    assert!(parse_sp3(b"#cP\n*  2024 01 01 00 00 0.0\nPG01 x 1 2\n").is_err());
}

#[test]
fn arbitrary_bytes_never_panic() {
    let mut x: u64 = 179;
    for _ in 0..200 {
        let len = (x % 300) as usize;
        let bytes: Vec<u8> = (0..len)
            .map(|_| {
                x ^= x << 13;
                x ^= x >> 7;
                x ^= x << 17;
                let alphabet = b"> *#PGEG0123456789.- \n";
                alphabet[(x % alphabet.len() as u64) as usize]
            })
            .collect();
        let _ = parse_rinex_obs(&bytes);
        let _ = parse_sp3(&bytes);
    }
}
