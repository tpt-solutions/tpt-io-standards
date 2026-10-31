#![no_main]
//! Fuzz target: feed arbitrary bytes to the FITS reader.
//!
//! Run with (nightly required):
//!
//! ```text
//! cargo +nightly fuzz run fits
//! ```
//!
//! The reader must never panic on malformed input; it returns `Ok` or `Err`.

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(file) = tpt_io_fits::parse(data) {
        for hdu in &file.hdus {
            let _ = hdu.pixels();
        }
        let _ = tpt_io_fits::write(&file);
    }
});
