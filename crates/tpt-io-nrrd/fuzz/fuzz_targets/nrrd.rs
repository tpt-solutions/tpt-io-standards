#![no_main]
//! Fuzz target: feed arbitrary bytes to the NRRD reader.
//!
//! Run with (nightly required):
//!
//! ```text
//! cargo +nightly fuzz run nrrd
//! ```
//!
//! The reader must never panic on malformed input; it returns `Ok` or `Err`.

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(n) = tpt_io_nrrd::parse(data) {
        let _ = n.values();
        let _ = tpt_io_nrrd::write(&n);
    }
});
