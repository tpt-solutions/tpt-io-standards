#![no_main]
//! Fuzz target: feed arbitrary bytes to the MTConnect devices and streams readers.
//!
//! Run with (nightly required):
//!
//! ```text
//! cargo +nightly fuzz run mtconnect
//! ```
//!
//! The readers must never panic on malformed input; they return `Ok` or `Err`.

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(devices) = tpt_io_mtconnect::parse_devices(data) {
        let _ = devices.validate();
    }
    if let Ok(streams) = tpt_io_mtconnect::parse_streams(data) {
        let _ = streams.observations.len();
    }
});
