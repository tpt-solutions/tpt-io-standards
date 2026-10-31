//! Integration tests for the HEC-RAS geometry reader.

use tpt_io_hecras::{parse, HecRasError};

const GEOMETRY: &[u8] = b"Geom Title=Demo
River Reach=Main River,Upper Reach
Reach XY= 3
 0.0 0.0
 500.0 120.0
 1000.0 240.0
Type RM Length L Ch R = 1 ,1000.5,50,50,50
#Sta/Elev= 3
 0 100
 50 90
 100 101
Type RM Length L Ch R = 1 ,500,40,40,40
#Sta/Elev= 2
 0,98
 80,99
River Reach=Tributary,Lower
Type RM Length L Ch R = 1 ,12.0,10,10,10
#Sta/Elev= 2
0 5
20 6
";

#[test]
fn reads_reaches_centrelines_and_cross_sections() {
    let g = parse(GEOMETRY).unwrap();
    assert_eq!(g.reaches.len(), 2);
    let main = &g.reaches[0];
    assert_eq!(main.river, "Main River");
    assert_eq!(main.reach, "Upper Reach");
    assert_eq!(main.centerline, vec![(0.0, 0.0), (500.0, 120.0), (1000.0, 240.0)]);
    assert_eq!(main.cross_sections.len(), 2);
    assert_eq!(main.cross_sections[0].river_station, 1000.5);
    assert_eq!(main.cross_sections[0].points[2], (100.0, 101.0));
    assert_eq!(main.cross_sections[1].points, vec![(0.0, 98.0), (80.0, 99.0)]);
    assert_eq!(g.reaches[1].cross_sections[0].river_station, 12.0);
}

#[test]
fn keywords_out_of_place_are_errors() {
    assert!(matches!(parse(b"Type RM Length L Ch R = 1 ,5,1,1,1\n"), Err(HecRasError::Invalid(_))));
    assert!(matches!(parse(b"River Reach=A,B\n#Sta/Elev= 1\n0 1\n"), Err(HecRasError::Invalid(_))));
    assert!(matches!(parse(b"River Reach=A,B\nReach XY= 5\n0 0\n"), Err(HecRasError::Invalid(_))));
    assert!(parse(b"River Reach=A,B\nReach XY= 1\nx y\n").is_err());
}

#[test]
fn arbitrary_bytes_never_panic() {
    let mut x: u64 = 173;
    for _ in 0..200 {
        let len = (x % 200) as usize;
        let bytes: Vec<u8> = (0..len)
            .map(|_| {
                x ^= x << 13;
                x ^= x >> 7;
                x ^= x << 17;
                let alphabet = b"River Reach=,Type RM #Sta/Elev= 0123456789.-\n";
                alphabet[(x % alphabet.len() as u64) as usize]
            })
            .collect();
        let _ = parse(&bytes);
    }
}
