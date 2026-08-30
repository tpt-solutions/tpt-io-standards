#![no_main]
//! Fuzz target: feed arbitrary bytes to the DICOM parser.
//!
//! Run with (nightly required):
//!
//! ```text
//! cargo +nightly fuzz run dicom
//! ```
//!
//! The parser must never panic on malformed input; it returns `Ok` or `Err`.

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Try parsing as a full DICOM file.
    if let Ok(file) = tpt_io_dicom::parser::parse_dicom(data, true) {
        // Exercise the parsed result.
        let _ = file.transfer_syntax;
        let _ = file.meta.len();
        let _ = file.dataset.len();
        // Try to extract pixel descriptor (may fail, that's fine).
        let _ = tpt_io_dicom::pixel::extract_pixel_descriptor(&file.dataset);
        // Try hierarchy navigation.
        let _ = tpt_io_dicom::hierarchy::HierarchyNavigator::new().study(&file.dataset);
        let _ = tpt_io_dicom::hierarchy::HierarchyNavigator::new().series(&file.dataset);
        let _ = tpt_io_dicom::hierarchy::HierarchyNavigator::new().instance(&file.dataset);
    }

    // Also try without preamble requirement.
    if let Ok(file) = tpt_io_dicom::parser::parse_dicom(data, false) {
        let _ = file.transfer_syntax;
        let _ = file.meta.len();
        let _ = file.dataset.len();
    }

    // Try parsing as a standalone dataset (explicit VR LE).
    if let Ok((ds, _)) = tpt_io_dicom::parser::parse_dataset(data, 0, true, true) {
        let _ = ds.len();
    }

    // Try parsing as a standalone dataset (implicit VR LE).
    if let Ok((ds, _)) = tpt_io_dicom::parser::parse_dataset(data, 0, false, true) {
        let _ = ds.len();
    }
});
