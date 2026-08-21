#![no_main]
//! Fuzz target: feed arbitrary bytes to the NIfTI parser.
//!
//! Run with (nightly required):
//!
//! ```text
//! cargo +nightly fuzz run nifti
//! ```
//!
//! The parser must never panic on malformed input; it returns `Ok` or `Err`.

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(f) = tpt_io_nifti::parser::parse_nii(data) {
        let _ = tpt_io_nifti::writer::round_trip(&f.header);
        let _ = f.voxel_count();
    }
});
