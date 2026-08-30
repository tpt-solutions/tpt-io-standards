#![no_main]
//! Fuzz target: feed arbitrary bytes to the IFC (SPF) parser.
//!
//! Run with (nightly required):
//!
//! ```text
//! cargo +nightly fuzz run ifc
//! ```
//!
//! The parser must never panic on malformed input; it returns `Ok` or `Err`.

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = core::str::from_utf8(data) {
        let _ = tpt_io_ifc::parser::parse_str(s);
    }
});
