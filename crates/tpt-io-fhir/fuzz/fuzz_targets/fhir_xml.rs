#![no_main]
//! Fuzz target: feed arbitrary bytes to the FHIR XML parser.
//!
//! Run with (nightly required):
//!
//! ```text
//! cargo +nightly fuzz run fhir_xml
//! ```
//!
//! The parser must never panic on malformed input; it returns `Ok` or `Err`.

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = tpt_io_fhir::xml::parse_resource(
        core::str::from_utf8(data).unwrap_or(""),
    );
});
