#![no_main]
//! Fuzz target: feed arbitrary bytes to the SBML parser.
//!
//! Run with (nightly required):
//!
//! ```text
//! cargo +nightly fuzz run sbml
//! ```
//!
//! The parser must never panic on malformed input; it returns `Ok` or `Err`.

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(sbml) = tpt_io_sbml::parse_sbml(data) {
        // Exercise the data model and validation paths on whatever parsed,
        // guarding against panics in the consistency checks.
        let _ = sbml.level;
        let _ = sbml.model.species.len();
        let _ = tpt_io_sbml::validate_sbml(&sbml);
        for r in &sbml.model.reactions {
            if let Some(kl) = &r.kinetic_law {
                if let Some(math) = &kl.math {
                    let _ = math.to_infix_string();
                }
            }
        }
    }
});
