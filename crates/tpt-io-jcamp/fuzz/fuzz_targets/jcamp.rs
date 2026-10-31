#![no_main]
//! Fuzz target: feed arbitrary bytes to the JCAMP-DX reader.
//!
//! Run with (nightly required):
//!
//! ```text
//! cargo +nightly fuzz run jcamp
//! ```
//!
//! The reader must never panic on malformed input; it returns `Ok` or `Err`.

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(spectrum) = tpt_io_jcamp::parse(data) {
        let _ = tpt_io_jcamp::write(&spectrum);
    }
});
