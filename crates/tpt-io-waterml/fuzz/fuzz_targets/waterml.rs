#![no_main]
//! Fuzz target: feed arbitrary bytes to the WaterML 2.0 reader.
//!
//! Run with (nightly required):
//!
//! ```text
//! cargo +nightly fuzz run waterml
//! ```
//!
//! The reader must never panic on malformed input; it returns `Ok` or `Err`.

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(series) = tpt_io_waterml::parse(data) {
        let _ = tpt_io_waterml::write(&series);
    }
});
