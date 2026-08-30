#![no_main]
//! Fuzz target for the CAPE-OPEN property-package text codec.
//!
//! Drives [`parse_property_package`] (and the validating [`PropertyPackageReader`])
//! with arbitrary bytes. Invalid UTF-8 / malformed input must fail gracefully
//! (returning an error) and never panic — that is what this target checks.

use libfuzzer_sys::fuzz_target;

use tpt_io_capeopen::parser::{parse_property_package, PropertyPackageReader};

fuzz_target!(|data: &[u8]| {
    // The free parser never panics on malformed input.
    let _ = parse_property_package(data);

    // The validating reader additionally runs semantic checks; still no panics.
    let _ = PropertyPackageReader::new().parse(data);
});
