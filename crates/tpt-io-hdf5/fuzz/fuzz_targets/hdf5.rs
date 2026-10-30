#![no_main]
//! Fuzz target: feed arbitrary bytes to the HDF5 reader.
//!
//! Run with (nightly required):
//!
//! ```text
//! cargo +nightly fuzz run hdf5
//! ```
//!
//! The reader must never panic on malformed input; it returns `Ok` or `Err`.
//! Whatever parses is also read back, so the data paths are exercised too.

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(file) = tpt_io_hdf5::parse(data) {
        for ds in &file.datasets {
            let _ = ds.read_data(data);
        }
    }
});
