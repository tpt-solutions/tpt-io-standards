#![no_main]
//! Fuzz target: feed arbitrary bytes to the WITSML trajectory reader.
//!
//! Run with (nightly required):
//!
//! ```text
//! cargo +nightly fuzz run witsml
//! ```
//!
//! The reader must never panic on malformed input; it returns `Ok` or `Err`.

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = tpt_io_witsml::parse(data);
});
