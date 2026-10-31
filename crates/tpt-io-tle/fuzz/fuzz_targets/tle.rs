#![no_main]
//! Fuzz target: feed arbitrary bytes to the TLE reader.
//!
//! Run with (nightly required):
//!
//! ```text
//! cargo +nightly fuzz run tle
//! ```
//!
//! The reader must never panic on malformed input; it returns `Ok` or `Err`.

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(sets) = tpt_io_tle::parse(data) {
        let _ = tpt_io_tle::write(&sets);
    }
});
