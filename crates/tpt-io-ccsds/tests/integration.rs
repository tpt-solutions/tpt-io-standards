//! Integration tests for the CCSDS OEM, OPM and TDM readers.

use tpt_io_ccsds::{parse_oem, parse_opm, parse_tdm, write_oem, CcsdsError, OemMeta, StateVector};

const OEM: &[u8] = b"CCSDS_OEM_VERS = 2.0
CREATION_DATE = 2024-01-01T00:00:00
ORIGINATOR = TEST

META_START
OBJECT_NAME = SAT-1
OBJECT_ID = 2024-001A
CENTER_NAME = EARTH
REF_FRAME = EME2000
TIME_SYSTEM = UTC
START_TIME = 2024-01-01T00:00:00
STOP_TIME = 2024-01-01T00:02:00
META_STOP
COMMENT first two states
2024-01-01T00:00:00 7000.0 0.0 0.0 0.0 7.5 0.0
2024-01-01T00:01:00 6999.5 450.1 0.0 -0.5 7.4 0.0
COVARIANCE_START
EPOCH = 2024-01-01T00:00:00
1.0 0.0 0.0 0.0 0.0 0.0
COVARIANCE_STOP

META_START
OBJECT_NAME = SAT-1
OBJECT_ID = 2024-001A
CENTER_NAME = EARTH
REF_FRAME = EME2000
TIME_SYSTEM = UTC
START_TIME = 2024-01-01T00:02:00
STOP_TIME = 2024-01-01T00:03:00
USEABLE_START_TIME = 2024-01-01T00:02:30
META_STOP
2024-01-01T00:02:00 6998.0 900.2 0.0 -1.0 7.3 0.0 0.0001 0.0 0.0
";

#[test]
fn reads_two_segments_with_states_and_covariance_skipped() {
    let oem = parse_oem(OEM).unwrap();
    assert_eq!(oem.version, "2.0");
    assert_eq!(oem.originator.as_deref(), Some("TEST"));
    assert_eq!(oem.segments.len(), 2);
    let first = &oem.segments[0];
    assert_eq!(first.meta.object_name, "SAT-1");
    assert_eq!(first.meta.ref_frame, "EME2000");
    assert_eq!(first.states.len(), 2);
    assert_eq!(first.states[1].position, [6999.5, 450.1, 0.0]);
    assert_eq!(first.states[1].velocity, [-0.5, 7.4, 0.0]);
    let second = &oem.segments[1];
    assert_eq!(
        second.meta.extra,
        [("USEABLE_START_TIME".to_string(), "2024-01-01T00:02:30".to_string())]
    );
    assert_eq!(second.states[0].epoch, "2024-01-01T00:02:00");
}

#[test]
fn write_then_parse_is_identity() {
    let oem = parse_oem(OEM).unwrap();
    let text = write_oem(&oem);
    assert_eq!(parse_oem(&text).unwrap(), oem);
}

#[test]
fn oem_writer_builds_a_message_from_values() {
    let oem = tpt_io_ccsds::Oem {
        version: "2.0".into(),
        creation_date: None,
        originator: None,
        segments: vec![tpt_io_ccsds::OemSegment {
            meta: OemMeta { object_name: "X".into(), ..OemMeta::default() },
            states: vec![StateVector {
                epoch: "t0".into(),
                position: [1.5, -2.25, 0.0],
                velocity: [0.0; 3],
            }],
        }],
    };
    let back = parse_oem(&write_oem(&oem)).unwrap();
    assert_eq!(back.segments[0].states[0].position, [1.5, -2.25, 0.0]);
}

#[test]
fn oem_rejects_missing_version_and_bad_numbers() {
    assert!(matches!(parse_oem(b"META_START\nMETA_STOP\n"), Err(CcsdsError::Invalid(_))));
    let bad = b"CCSDS_OEM_VERS = 2.0\nMETA_START\nMETA_STOP\n2024 1 2 three 4 5 6\n";
    assert!(parse_oem(bad).is_err());
    let outside = b"CCSDS_OEM_VERS = 2.0\n2024 1 2 3 4 5 6\n";
    assert!(parse_oem(outside).is_err());
}

const OPM: &[u8] = b"CCSDS_OPM_VERS = 2.0
CREATION_DATE = 2024-03-01T12:00:00
OBJECT_NAME = SAT-2
META_START
REF_FRAME = EME2000
META_STOP
EPOCH = 2024-03-01T12:00:00
X = 7000.0
Y = 0.0
Z = 0.0
X_DOT = 0.0
Y_DOT = 7.5
Z_DOT = 0.1
SEMI_MAJOR_AXIS = 7000.0
";

#[test]
fn opm_reads_fields_and_state_vector() {
    let opm = parse_opm(OPM).unwrap();
    assert_eq!(opm.version, "2.0");
    assert_eq!(opm.get("OBJECT_NAME"), Some("SAT-2"));
    assert_eq!(opm.get("SEMI_MAJOR_AXIS"), Some("7000.0"));
    let s = opm.state.unwrap();
    assert_eq!(s.epoch, "2024-03-01T12:00:00");
    assert_eq!(s.velocity, [0.0, 7.5, 0.1]);
}

#[test]
fn opm_without_a_full_state_has_none() {
    let opm = parse_opm(b"CCSDS_OPM_VERS = 2.0\nX = 1\n").unwrap();
    assert!(opm.state.is_none());
}

const TDM: &[u8] = b"CCSDS_TDM_VERS = 2.0
CREATION_DATE = 2024-04-01T00:00:00
META_START
TIME_SYSTEM = UTC
PARTICIPANT_1 = STATION-A
META_STOP
DATA_START
TRACKING_DATA = RANGE
RANGE_UNITS = km
DATA_STOP
";

#[test]
fn tdm_keeps_header_metadata_and_data_blocks() {
    let tdm = parse_tdm(TDM).unwrap();
    assert_eq!(tdm.version, "2.0");
    assert_eq!(tdm.header[0], ("CCSDS_TDM_VERS".to_string(), "2.0".to_string()));
    assert_eq!(tdm.segments.len(), 1);
    let (meta, data) = &tdm.segments[0];
    assert_eq!(meta[1], ("PARTICIPANT_1".to_string(), "STATION-A".to_string()));
    assert_eq!(data[1], ("RANGE_UNITS".to_string(), "km".to_string()));
}

#[test]
fn tdm_data_stop_without_start_is_an_error() {
    assert!(parse_tdm(b"CCSDS_TDM_VERS = 2.0\nDATA_STOP\n").is_err());
    assert!(parse_tdm(b"TIME_SYSTEM = UTC\n").is_err());
}

#[test]
fn arbitrary_bytes_never_panic() {
    let mut x: u64 = 17;
    for _ in 0..400 {
        let len = (x % 200) as usize;
        let bytes: Vec<u8> = (0..len)
            .map(|_| {
                x ^= x << 13;
                x ^= x >> 7;
                x ^= x << 17;
                b"META_STARTSTOP=12 .\nEPOCH_xyz\nCOMMENT"[(x % 36) as usize]
            })
            .collect();
        let _ = parse_oem(&bytes);
        let _ = parse_opm(&bytes);
        let _ = parse_tdm(&bytes);
    }
}
