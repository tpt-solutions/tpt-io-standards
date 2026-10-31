#![no_main]
//! Fuzz target: feed arbitrary bytes to the EDF/BDF reader.
//!
//! Run with (nightly required):
//!
//! ```text
//! cargo +nightly fuzz run edf
//! ```
//!
//! The reader must never panic on malformed input; it returns `Ok` or `Err`.

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(e) = tpt_io_edf::parse(data) {
        for i in 0..e.signals.len() {
            let _ = e.physical(i);
        }
        let _ = tpt_io_edf::write(&e);
    }
});
