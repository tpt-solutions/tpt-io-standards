#![no_main]
//! Fuzz target: feed arbitrary bytes to the MATPOWER case reader.
//!
//! Run with (nightly required):
//!
//! ```text
//! cargo +nightly fuzz run matpower
//! ```
//!
//! The reader must never panic on malformed input; it returns `Ok` or `Err`.

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(case) = tpt_io_matpower::parse(data) {
        let _ = case.validate();
        let _ = tpt_io_matpower::write(&case);
    }
});
