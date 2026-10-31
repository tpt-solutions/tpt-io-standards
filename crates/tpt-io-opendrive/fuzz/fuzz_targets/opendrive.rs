#![no_main]
//! Fuzz target: feed arbitrary bytes to the OpenDRIVE reader.
//!
//! Run with (nightly required):
//!
//! ```text
//! cargo +nightly fuzz run opendrive
//! ```
//!
//! The reader must never panic on malformed input; it returns `Ok` or `Err`.

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(map) = tpt_io_opendrive::parse(data) {
        let _ = map.validate();
        let _ = tpt_io_opendrive::write(&map);
    }
});
