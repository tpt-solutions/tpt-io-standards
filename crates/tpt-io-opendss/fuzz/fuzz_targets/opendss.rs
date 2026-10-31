#![no_main]
//! Fuzz target: feed arbitrary bytes to the OpenDSS script reader.
//!
//! Run with (nightly required):
//!
//! ```text
//! cargo +nightly fuzz run opendss
//! ```
//!
//! The reader must never panic on malformed input; it returns `Ok` or `Err`.

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(script) = tpt_io_opendss::parse(data) {
        let _ = tpt_io_opendss::write(&script);
    }
});
