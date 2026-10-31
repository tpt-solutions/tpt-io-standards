#![no_main]
//! Fuzz target: feed arbitrary bytes to the 3MF package reader.
//!
//! Run with (nightly required):
//!
//! ```text
//! cargo +nightly fuzz run threemf
//! ```
//!
//! The reader must never panic on malformed input; it returns `Ok` or `Err`.

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = tpt_io_threemf::parse(data);
    let _ = tpt_io_threemf::parse_model_xml(data);
});
