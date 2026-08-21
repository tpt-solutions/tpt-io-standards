#![no_main]
//! Fuzz target: feed arbitrary bytes to the EDIF parser and flattener.
//!
//! Run with (nightly required):
//!
//! ```text
//! cargo +nightly fuzz run edif
//! ```
//!
//! The parser and flattener must never panic on malformed input; they return
//! `Ok` or `Err`.

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = core::str::from_utf8(data) {
        if let Ok(edif) = tpt_io_edif::parser::parse_edif(s.as_bytes()) {
            let _ = tpt_io_edif::flatten::flatten(&edif);
            let _ = tpt_io_edif::writer::serialize(&edif);
        }
    }
});
