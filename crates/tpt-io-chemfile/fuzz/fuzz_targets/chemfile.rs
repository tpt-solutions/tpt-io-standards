#![no_main]
//! Fuzz target: feed arbitrary bytes to the XYZ, cube, POSCAR and LAMMPS readers.
//!
//! Run with (nightly required):
//!
//! ```text
//! cargo +nightly fuzz run chemfile
//! ```
//!
//! The readers must never panic on malformed input; they return `Ok` or `Err`.

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = tpt_io_chemfile::xyz::parse(data);
    let _ = tpt_io_chemfile::cube::parse(data);
    let _ = tpt_io_chemfile::poscar::parse(data);
    let _ = tpt_io_chemfile::lammps::parse(data);
});
