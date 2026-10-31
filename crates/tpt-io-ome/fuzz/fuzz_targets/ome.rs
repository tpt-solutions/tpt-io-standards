#![no_main]
//! Fuzz target: feed arbitrary bytes to the OME-XML, OME-TIFF and OME-Zarr readers.
//!
//! Run with (nightly required):
//!
//! ```text
//! cargo +nightly fuzz run ome
//! ```
//!
//! The readers must never panic on malformed input; they return `Ok` or `Err`.

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = tpt_io_ome::parse_xml(data);
    let _ = tpt_io_ome::from_tiff(data);
    let _ = tpt_io_ome::parse_zattrs(data);
    let _ = tpt_io_ome::parse_zarray(data);
});
