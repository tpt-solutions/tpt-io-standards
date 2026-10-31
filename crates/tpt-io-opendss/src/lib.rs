//! OpenDSS scripts (`.dss`): reader, writer and element queries.
//!
//! OpenDSS describes distribution circuits as a sequence of text commands such as
//! `New Circuit.feeder basekv=12.47` or `New Line.L1 bus1=a bus2=b length=0.5`. This crate
//! splits a script into [`Command`]s, keeps the raw property values (so unusual syntax such
//! as `[1 2 3]` arrays round-trips unchanged), and writes the commands back out.
//!
//! ```
//! let script = b"Clear\nNew Circuit.feeder basekv=12.47\nNew Line.L1 bus1=a bus2=b length=0.5\n";
//! let dss = tpt_io_opendss::parse(script).unwrap();
//! let lines: Vec<_> = dss.elements("Line").collect();
//! assert_eq!(lines[0].name.as_deref(), Some("L1"));
//! assert_eq!(lines[0].prop("bus2"), Some("b"));
//! ```
//!
//! Supported syntax: `!` and `//` comments, `~` continuation lines, quoted values, and
//! brackets or parentheses containing spaces. Command semantics (power flow, load
//! allocation) are not evaluated; this crate reads the script, not the circuit.
//!
//! Spec access: the OpenDSS command reference is published by EPRI under a BSD-style
//! licence. No manual text is copied into this crate.

#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]

#[cfg(any(feature = "alloc", feature = "std"))]
#[allow(unused_imports)]
#[macro_use]
extern crate alloc;

pub mod error;
pub mod script;

pub use error::OpenDssError;
pub use script::{parse, write, Command, Script};
