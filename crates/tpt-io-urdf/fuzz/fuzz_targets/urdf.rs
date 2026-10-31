#![no_main]
//! Fuzz target: feed arbitrary bytes to the URDF and SDFormat reader.
//!
//! Run with (nightly required):
//!
//! ```text
//! cargo +nightly fuzz run urdf
//! ```
//!
//! The reader must never panic on malformed input; it returns `Ok` or `Err`.

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(robot) = tpt_io_urdf::parse(data) {
        let _ = robot.validate();
        let _ = tpt_io_urdf::write(&robot);
    }
});
