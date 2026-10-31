#![no_main]
//! Fuzz target: feed arbitrary bytes to the DAF reader, then read every segment the summaries describe.
//!
//! Run with (nightly required):
//!
//! ```text
//! cargo +nightly fuzz run naif
//! ```
//!
//! The reader must never panic on malformed input; it returns `Ok` or `Err`.

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(daf) = tpt_io_naif::Daf::parse(data) {
        for s in &daf.summaries {
            let _ = daf.segment(s);
        }
    }
});
