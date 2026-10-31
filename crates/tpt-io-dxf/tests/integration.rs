//! Integration tests for the DXF reader and writer.

use tpt_io_dxf::{parse, write, DxfError, Drawing, Entity};

const DRAWING: &[u8] = b"0
SECTION
2
HEADER
9
$ACADVER
1
AC1027
0
ENDSEC
0
SECTION
2
TABLES
0
TABLE
2
LAYER
0
LAYER
2
Walls
0
LAYER
2
Text
0
ENDTAB
0
ENDSEC
0
SECTION
2
ENTITIES
0
LINE
8
Walls
10
0.0
20
0.0
30
0.0
11
5.5
21
2.0
31
0.0
0
CIRCLE
8
Walls
10
1.0
20
1.0
40
2.5
0
LWPOLYLINE
8
Walls
90
3
70
1
10
0
20
0
10
4
20
0
10
4
20
3
0
TEXT
8
Text
10
2
20
3
40
0.5
1
Hello
0
HATCH
8
Walls
0
ENDSEC
0
EOF
";

#[test]
fn reads_version_layers_and_entities() {
    let d = parse(DRAWING).unwrap();
    assert_eq!(d.version.as_deref(), Some("AC1027"));
    assert_eq!(d.layers, ["Walls", "Text"]);
    assert_eq!(d.entities.len(), 5);
    match &d.entities[0] {
        Entity::Line { layer, start, end } => {
            assert_eq!(layer, "Walls");
            assert_eq!(*start, [0.0, 0.0, 0.0]);
            assert_eq!(*end, [5.5, 2.0, 0.0]);
        }
        other => panic!("expected a line, got {other:?}"),
    }
}

#[test]
fn circles_polylines_text_and_other_entities() {
    let d = parse(DRAWING).unwrap();
    assert_eq!(d.entities[1], Entity::Circle { layer: "Walls".into(), center: [1.0, 1.0, 0.0], radius: 2.5 });
    assert_eq!(
        d.entities[2],
        Entity::LwPolyline {
            layer: "Walls".into(),
            closed: true,
            vertices: vec![[0.0, 0.0], [4.0, 0.0], [4.0, 3.0]],
        }
    );
    assert_eq!(
        d.entities[3],
        Entity::Text { layer: "Text".into(), insert: [2.0, 3.0, 0.0], height: 0.5, value: "Hello".into() }
    );
    assert_eq!(d.entities[4], Entity::Other { kind: "HATCH".into(), layer: "Walls".into() });
}

#[test]
fn write_then_parse_is_identity() {
    let d = parse(DRAWING).unwrap();
    let text = write(&d);
    assert_eq!(parse(&text).unwrap(), d);
}

#[test]
fn a_drawing_built_in_code_round_trips() {
    let d = Drawing {
        version: None,
        layers: vec!["0".into()],
        entities: vec![Entity::Circle { layer: "0".into(), center: [0.0; 3], radius: 1.0 }],
    };
    assert_eq!(parse(&write(&d)).unwrap(), d);
}

#[test]
fn malformed_streams_are_errors() {
    assert!(matches!(parse(b"0\nSECTION\n2"), Err(DxfError::Invalid(_))));
    assert!(parse(b"x\nSECTION\n").is_err());
    assert!(parse(b"0\nSECTION\n2\nENTITIES\n0\nLINE\n8\n0\n10\nabc\n").is_err());
}

#[test]
fn arbitrary_bytes_never_panic() {
    let mut x: u64 = 59;
    for _ in 0..300 {
        let len = (x % 200) as usize;
        let bytes: Vec<u8> = (0..len)
            .map(|_| {
                x ^= x << 13;
                x ^= x >> 7;
                x ^= x << 17;
                let alphabet = b"0123456789.-\nSECTIONLINEENDSEC ";
                alphabet[(x % alphabet.len() as u64) as usize]
            })
            .collect();
        let _ = parse(&bytes);
    }
}
