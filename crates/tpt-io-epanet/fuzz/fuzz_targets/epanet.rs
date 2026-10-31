#![no_main]
//! Fuzz target: feed arbitrary bytes to the EPANET `.inp` reader.
//!
//! Run with (nightly required):
//!
//! ```text
//! cargo +nightly fuzz run epanet
//! ```
//!
//! The reader must never panic on malformed input; it returns `Ok` or `Err`.

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(inp) = tpt_io_epanet::parse(data) {
        let _ = inp.validate();
        let _ = tpt_io_epanet::write(&inp);
    }
});
