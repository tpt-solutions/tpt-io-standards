#![no_main]
//! Fuzz target: feed arbitrary bytes to the FHIR JSON parser.
//!
//! Run with (nightly required):
//!
//! ```text
//! cargo +nightly fuzz run fhir_json
//! ```
//!
//! The parser must never panic on malformed input; it returns `Ok` or `Err`.

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = tpt_io_fhir::json::parse_resource(
        core::str::from_utf8(data).unwrap_or(""),
    );
});
