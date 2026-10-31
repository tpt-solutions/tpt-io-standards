//! Integration tests for the CIF / mmCIF reader and writer.

use tpt_io_cif::{parse, write, CifError};

const QUARTZ: &[u8] = b"#\\#CIF_1.1
data_quartz
_chemical_name_mineral 'Alpha quartz'
_cell_length_a 4.9134
_cell_length_b   4.9134
_symmetry_space_group_name_H-M 'P 32 2 1'
_publ_section_title
;
Refinement of quartz
  at two temperatures
;
loop_
_atom_site_label
_atom_site_type_symbol
_atom_site_fract_x
_atom_site_fract_y
Si1 Si 0.4697 0.0000
O1  O  0.4135 0.2669
'O 2' O 0.1234 \"0.5\"
data_second
_cell_length_a 10
";

#[test]
fn reads_blocks_items_and_loops() {
    let f = parse(QUARTZ).unwrap();
    assert_eq!(f.blocks.len(), 2);
    let q = &f.blocks[0];
    assert_eq!(q.name, "quartz");
    assert_eq!(q.item("_cell_length_a"), Some("4.9134"));
    assert_eq!(q.item("_chemical_name_mineral"), Some("'Alpha quartz'"));
    assert_eq!(q.item("_publ_section_title"), Some("Refinement of quartz\n  at two temperatures"));
    let lp = q.loop_with("_atom_site_fract_x").unwrap();
    assert_eq!(lp.tags.len(), 4);
    assert_eq!(lp.rows.len(), 3);
    assert_eq!(lp.rows[1][0], "O1");
    assert_eq!(lp.rows[2][0], "'O 2'");
    assert_eq!(f.blocks[1].item("_cell_length_a"), Some("10"));
}

#[test]
fn write_then_parse_is_identity() {
    let f = parse(QUARTZ).unwrap();
    let text = write(&f);
    assert_eq!(parse(&text).unwrap(), f);
}

#[test]
fn lookups_are_case_insensitive() {
    let f = parse(QUARTZ).unwrap();
    assert_eq!(f.blocks[0].item("_CELL_LENGTH_A"), Some("4.9134"));
}

#[test]
fn malformed_input_is_an_error() {
    assert!(parse(b"_cell_length_a 1\n").is_err());
    assert!(parse(b"data_x\n_tag\n").is_err());
    assert!(parse(b"data_x\nloop_\n_a\n_b\n1 2 3\n").is_err());
    assert!(parse(b"data_x\n_t ;\nopen text\n").is_err());
    assert!(parse(b"data_x\n_t 'open\n").is_err());
    assert!(matches!(parse(b"loop_\n_a\n"), Err(CifError::Invalid(_))));
}

#[test]
fn arbitrary_bytes_never_panic() {
    let mut x: u64 = 37;
    for _ in 0..400 {
        let len = (x % 180) as usize;
        let bytes: Vec<u8> = (0..len)
            .map(|_| {
                x ^= x << 13;
                x ^= x >> 7;
                x ^= x << 17;
                let alphabet = b"data_loop_ _a\n;'\" 1.#";
                alphabet[(x % alphabet.len() as u64) as usize]
            })
            .collect();
        let _ = parse(&bytes);
    }
}
