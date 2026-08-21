#![no_main]

use libfuzzer_sys::fuzz_target;

/// Fuzz the explicit-VR little-endian dataset parser: malformed input must
/// produce errors, never panics.
fuzz_target!(|data: &[u8]| {
    let _ = tpt_io_dicom::dataset::parse(data, tpt_io_dicom::uid::TransferSyntax::ExplicitVrLittleEndian);
});