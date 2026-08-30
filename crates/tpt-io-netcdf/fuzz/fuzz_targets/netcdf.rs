#![no_main]
//! Fuzz target: feed arbitrary bytes to the NetCDF parser.
//!
//! Run with (nightly required):
//!
//! ```text
//! cargo +nightly fuzz run netcdf
//! ```
//!
//! The parser must never panic on malformed input; it returns `Ok` or `Err`.

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(nc) = tpt_io_netcdf::parser::parse_netcdf(data) {
        // Exercise the data-access helpers on whatever parsed, guarding the
        // (rare) record-variable path which is interleaved on disk.
        for v in &nc.vars {
            if !nc.is_record_var(v) {
                let _ = nc.var_scalar(data, v, 0);
                let _ = tpt_io_netcdf::writer::round_trip(&nc);
            }
        }
    }
    // NetCDF-4 detection + superblock parsing must also not panic.
    if tpt_io_netcdf::detect_format(data) == Some(tpt_io_netcdf::FileKind::NetCdf4) {
        let _ = tpt_io_netcdf::parse_hdf5_superblock(data);
    }
});
