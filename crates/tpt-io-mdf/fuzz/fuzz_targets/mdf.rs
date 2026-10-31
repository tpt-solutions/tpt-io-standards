#![no_main]
//! Fuzz target: feed arbitrary bytes to the MDF 4 reader.
//!
//! Run with (nightly required):
//!
//! ```text
//! cargo +nightly fuzz run mdf
//! ```
//!
//! The reader must never panic on malformed input; it returns `Ok` or `Err`.

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = tpt_io_mdf::parse(data);
});
