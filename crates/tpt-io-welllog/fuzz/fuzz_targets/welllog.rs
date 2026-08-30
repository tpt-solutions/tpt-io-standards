#![no_main]
//! Fuzz target: feed arbitrary bytes to the LAS/DLIS parsers.
//!
//! Run with (nightly required):
//!
//! ```text
//! cargo +nightly fuzz run welllog
//! ```
//!
//! The parsers must never panic on malformed input; they return `Ok` or `Err`.

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = tpt_io_welllog::binary::parse(data);
    let _ = tpt_io_welllog::dlis::parse(data);
    if let Ok(s) = core::str::from_utf8(data) {
        let _ = tpt_io_welllog::parser::parse_str(s);
    }
});
