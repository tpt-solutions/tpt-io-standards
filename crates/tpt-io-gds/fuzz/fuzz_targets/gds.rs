#![no_main]
//! Fuzz target: feed arbitrary bytes to the GDSII reader.
//!
//! Run with (nightly required):
//!
//! ```text
//! cargo +nightly fuzz run gds
//! ```
//!
//! The reader must never panic on malformed input; it returns `Ok` or `Err`.

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(lib) = tpt_io_gds::parse(data) {
        let _ = tpt_io_gds::write(&lib);
    }
});
