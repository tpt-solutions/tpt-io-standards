#![no_main]
//! Fuzz target: feed arbitrary bytes to the RINEX 3 observation and SP3 readers.
//!
//! Run with (nightly required):
//!
//! ```text
//! cargo +nightly fuzz run gnss
//! ```
//!
//! The readers must never panic on malformed input; they return `Ok` or `Err`.

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = tpt_io_gnss::parse_rinex_obs(data);
    let _ = tpt_io_gnss::parse_sp3(data);
});
