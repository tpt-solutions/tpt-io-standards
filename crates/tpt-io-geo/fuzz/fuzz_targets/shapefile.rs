#![no_main]
//! Fuzz target: feed arbitrary bytes to the Shapefile (.shp) parser and streaming
//! reader.
//!
//! ```text
//! cargo +nightly fuzz run shapefile
//! ```
//!
//! Must never panic on malformed input.

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(sf) = tpt_io_geo::shapefile::parse_shp(data) {
        let _ = sf.records.len();
    }
    let mut stream = tpt_io_geo::stream::ShapefileRecordStream::new(data);
    while let Some(r) = stream.next() {
        if r.is_err() {
            break;
        }
    }
});
