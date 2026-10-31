#![no_main]
//! Fuzz target: feed arbitrary bytes to the structural Verilog reader.
//!
//! Run with (nightly required):
//!
//! ```text
//! cargo +nightly fuzz run verilog
//! ```
//!
//! The reader must never panic on malformed input; it returns `Ok` or `Err`.

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(netlist) = tpt_io_verilog::parse(data) {
        let _ = tpt_io_verilog::write(&netlist);
    }
});
