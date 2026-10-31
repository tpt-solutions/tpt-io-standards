#![no_main]
//! Fuzz target: feed arbitrary bytes to the G-code reader.
//!
//! Run with (nightly required):
//!
//! ```text
//! cargo +nightly fuzz run gcode
//! ```
//!
//! The reader must never panic on malformed input; it returns `Ok` or `Err`.

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(program) = tpt_io_gcode::parse(data) {
        let _ = program.motion_summary();
        let _ = tpt_io_gcode::write(&program);
    }
});
