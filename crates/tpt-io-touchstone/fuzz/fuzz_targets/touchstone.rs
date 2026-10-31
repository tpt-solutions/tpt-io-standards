#![no_main]
//! Fuzz target: feed arbitrary bytes to the Touchstone reader.
//!
//! Run with (nightly required):
//!
//! ```text
//! cargo +nightly fuzz run touchstone
//! ```
//!
//! The reader must never panic on malformed input; it returns `Ok` or `Err`.

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    for ports in 1..=3 {
        if let Ok(file) = tpt_io_touchstone::parse(data, ports) {
            let _ = tpt_io_touchstone::write(&file);
        }
    }
});
