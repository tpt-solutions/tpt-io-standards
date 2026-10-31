#![no_main]
//! Fuzz target: feed arbitrary bytes to the GLM reader.
//!
//! Run with (nightly required):
//!
//! ```text
//! cargo +nightly fuzz run gridlabd
//! ```
//!
//! The reader must never panic on malformed input; it returns `Ok` or `Err`.

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(glm) = tpt_io_gridlabd::parse(data) {
        let _ = glm.validate();
        let _ = tpt_io_gridlabd::write(&glm);
    }
});
