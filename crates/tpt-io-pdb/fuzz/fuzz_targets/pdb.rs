#![no_main]
//! Fuzz target: feed arbitrary bytes to the PDB reader.
//!
//! Run with (nightly required):
//!
//! ```text
//! cargo +nightly fuzz run pdb
//! ```
//!
//! The reader must never panic on malformed input; it returns `Ok` or `Err`.

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(pdb) = tpt_io_pdb::parse(data) {
        let _ = tpt_io_pdb::write(&pdb);
    }
});
