#![no_main]
//! Fuzz target: feed arbitrary bytes to the OPC UA NodeSet reader.
//!
//! Run with (nightly required):
//!
//! ```text
//! cargo +nightly fuzz run opcua
//! ```
//!
//! The reader must never panic on malformed input; it returns `Ok` or `Err`.

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(set) = tpt_io_opcua::parse(data) {
        let _ = set.validate();
    }
});
