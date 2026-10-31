#![no_main]
//! Fuzz target: feed arbitrary bytes to the HL7 v2 reader.
//!
//! Run with (nightly required):
//!
//! ```text
//! cargo +nightly fuzz run hl7v2
//! ```
//!
//! The reader must never panic on malformed input; it returns `Ok` or `Err`.

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(message) = tpt_io_hl7v2::parse(data) {
        let _ = tpt_io_hl7v2::write(&message);
    }
});
