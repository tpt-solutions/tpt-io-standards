#![no_main]
//! Fuzz target: feed arbitrary bytes to the AAS JSON reader.
//!
//! Run with (nightly required):
//!
//! ```text
//! cargo +nightly fuzz run aas
//! ```
//!
//! The reader must never panic on malformed input; it returns `Ok` or `Err`.

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(env) = tpt_io_aas::parse(data) {
        let _ = env.validate();
    }
});
