#![no_main]
//! Fuzz target: feed arbitrary bytes to the DXF reader.
//!
//! Run with (nightly required):
//!
//! ```text
//! cargo +nightly fuzz run dxf
//! ```
//!
//! The reader must never panic on malformed input; it returns `Ok` or `Err`.

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(drawing) = tpt_io_dxf::parse(data) {
        let _ = tpt_io_dxf::write(&drawing);
    }
});
