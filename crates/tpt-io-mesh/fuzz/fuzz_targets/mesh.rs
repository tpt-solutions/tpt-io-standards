#![no_main]
//! Fuzz target: feed arbitrary bytes to the STL, OBJ and PLY readers.
//!
//! Run with (nightly required):
//!
//! ```text
//! cargo +nightly fuzz run mesh
//! ```
//!
//! The readers must never panic on malformed input; they return `Ok` or `Err`.

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = tpt_io_mesh::stl::parse(data);
    let _ = tpt_io_mesh::obj::parse(data);
    let _ = tpt_io_mesh::ply::parse(data);
});
