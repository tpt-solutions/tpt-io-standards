#![no_main]
//! Fuzz target: feed arbitrary bytes to the molfile and SDF readers.
//!
//! Run with (nightly required):
//!
//! ```text
//! cargo +nightly fuzz run chem
//! ```
//!
//! The readers must never panic on malformed input; they return `Ok` or `Err`.

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(records) = tpt_io_chem::parse_sdf(data) {
        let _ = tpt_io_chem::write_sdf(&records);
    }
    let _ = tpt_io_chem::parse_mol(data);
});
