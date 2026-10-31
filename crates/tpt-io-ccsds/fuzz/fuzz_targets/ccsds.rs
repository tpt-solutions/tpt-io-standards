#![no_main]
//! Fuzz target: feed arbitrary bytes to the CCSDS OEM, OPM and TDM readers.
//!
//! Run with (nightly required):
//!
//! ```text
//! cargo +nightly fuzz run ccsds
//! ```
//!
//! The readers must never panic on malformed input; they return `Ok` or `Err`.

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(oem) = tpt_io_ccsds::parse_oem(data) {
        let _ = tpt_io_ccsds::write_oem(&oem);
    }
    let _ = tpt_io_ccsds::parse_opm(data);
    let _ = tpt_io_ccsds::parse_tdm(data);
});
