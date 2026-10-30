#![no_main]
//! Fuzz target: feed arbitrary bytes to the GeoTIFF metadata extractor.
//!
//! ```text
//! cargo +nightly fuzz run geotiff
//! ```
//!
//! Must never panic on malformed input.

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = tpt_io_geo::geotiff::parse_geotiff(data);
});
