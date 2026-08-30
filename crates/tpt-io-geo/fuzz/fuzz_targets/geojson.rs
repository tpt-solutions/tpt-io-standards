#![no_main]
//! Fuzz target: feed arbitrary bytes to the GeoJSON parser + streaming reader.
//!
//! Run with (nightly required):
//!
//! ```text
//! cargo +nightly fuzz run geojson
//! ```
//!
//! The parser must never panic on malformed input; it returns `Ok` or `Err`.

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(fc) = tpt_io_geo::geojson::parse(data) {
        // Re-serialise a successfully parsed collection; still must not panic.
        let _ = tpt_io_geo::geojson::to_string(&fc);
    }
    let mut stream = tpt_io_geo::stream::GeoJsonFeatureStream::new(data);
    if let Ok(mut s) = stream {
        while let Some(r) = s.next() {
            if r.is_err() {
                break;
            }
        }
    }
});
