#![no_main]
//! Fuzz target: feed arbitrary bytes to the LandXML reader.
//!
//! Run with (nightly required):
//!
//! ```text
//! cargo +nightly fuzz run landxml
//! ```
//!
//! The reader must never panic on malformed input; it returns `Ok` or `Err`.

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(land) = tpt_io_landxml::parse(data) {
        let _ = land.validate();
        let _ = tpt_io_landxml::write(&land);
    }
});
