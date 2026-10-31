#![no_main]
//! Fuzz target: feed arbitrary bytes to the COMBINE archive reader and the SED-ML summary reader.
//!
//! Run with (nightly required):
//!
//! ```text
//! cargo +nightly fuzz run omex
//! ```
//!
//! Both readers must never panic on malformed input; they return `Ok` or `Err`.

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = tpt_io_omex::parse(data);
    let _ = tpt_io_omex::sedml::parse_sedml(data);
});
