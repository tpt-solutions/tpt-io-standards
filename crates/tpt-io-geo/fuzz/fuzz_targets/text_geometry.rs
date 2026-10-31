#![no_main]
//! Fuzz target: feed arbitrary bytes to the WKT, WKB, KML and GPX readers.
//!
//! Run with (nightly required):
//!
//! ```text
//! cargo +nightly fuzz run text_geometry
//! ```
//!
//! Each reader must never panic on malformed input; it returns `Ok` or `Err`.

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(text) = core::str::from_utf8(data) {
        let _ = tpt_io_geo::wkt::parse_wkt(text);
    }
    let _ = tpt_io_geo::wkb::parse_wkb(data);
    let _ = tpt_io_geo::kml::parse(data);
    let _ = tpt_io_geo::gpx::parse(data);
});
