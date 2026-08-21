#![no_main]
//! Fuzz target: feed arbitrary bytes to the SPICE parser.
//!
//! Run with (nightly required):
//!
//! ```text
//! cargo +nightly fuzz run spice
//! ```
//!
//! The parser must never panic on malformed input; it returns `Ok` or `Err`.

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = core::str::from_utf8(data) {
        let _ = tpt_io_spice::parser::parse_str(s);
        let net = tpt_io_spice::parser::parse_str(s).unwrap_or_default();
        let _ = tpt_io_spice::expand::expand_netlist(&net.elements, &net.subckts);
    }
});
