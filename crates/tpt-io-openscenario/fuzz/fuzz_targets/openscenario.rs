#![no_main]
//! Fuzz target: feed arbitrary bytes to the OpenSCENARIO reader.
//!
//! Run with (nightly required):
//!
//! ```text
//! cargo +nightly fuzz run openscenario
//! ```
//!
//! The reader must never panic on malformed input; it returns `Ok` or `Err`.

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(scenario) = tpt_io_openscenario::parse(data) {
        let _ = scenario.validate();
    }
});
