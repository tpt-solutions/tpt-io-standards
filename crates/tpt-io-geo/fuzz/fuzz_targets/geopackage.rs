#![no_main]
//! Fuzz target: feed arbitrary bytes to the SQLite, GeoPackage and FlatGeobuf readers.
//!
//! Run with (nightly required):
//!
//! ```text
//! cargo +nightly fuzz run geopackage
//! ```
//!
//! The readers must never panic on malformed input; they return `Ok` or `Err`.

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = tpt_io_geo::geopackage::layers(data);
    let _ = tpt_io_geo::geopackage::read_features(data, "cities");
    let _ = tpt_io_geo::flatgeobuf::read(data);
});
