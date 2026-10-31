#![no_main]
//! Fuzz target: feed arbitrary bytes to the SEG-Y reader.
//!
//! Run with (nightly required):
//!
//! ```text
//! cargo +nightly fuzz run segy
//! ```
//!
//! The reader must never panic on malformed input; it returns `Ok` or `Err`.

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(file) = tpt_io_segy::parse(data) {
        let _ = tpt_io_segy::write(&file);
    }
});
