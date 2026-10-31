//! CCSDS orbit data messages in KVN (key = value) form: OEM, OPM and TDM.
//!
//! - **OEM** (Orbit Ephemeris Message): one or more segments, each with a metadata block and a list
//!   of Cartesian state vectors. Read and written, so round trips are exact.
//! - **OPM** (Orbit Parameter Message): a single state vector and its header and metadata fields,
//!   kept as ordered key/value pairs. Read only.
//! - **TDM** (Tracking Data Message): metadata and tracking data blocks, kept as ordered key/value
//!   pairs. Read only; observation grouping is not interpreted.
//!
//! ```
//! let oem = b"CCSDS_OEM_VERS = 2.0
//! META_START
//! OBJECT_NAME = SAT-1
//! OBJECT_ID = 2024-001A
//! CENTER_NAME = EARTH
//! REF_FRAME = EME2000
//! TIME_SYSTEM = UTC
//! START_TIME = 2024-01-01T00:00:00
//! STOP_TIME = 2024-01-01T00:01:00
//! META_STOP
//! 2024-01-01T00:00:00 7000.0 0.0 0.0 0.0 7.5 0.0
//! ";
//! let eph = tpt_io_ccsds::parse_oem(oem).unwrap();
//! assert_eq!(eph.segments[0].states[0].position[0], 7000.0);
//! ```
//!
//! Units follow the standard: positions in kilometres, velocities in kilometres per second.
//! Epochs are kept as the text in the file.
//!
//! Spec access: the CCSDS orbit data messages are published by the Consultative Committee for
//! Space Data Systems under its open standards terms. No recommendation text is copied here.

#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]

#[cfg(any(feature = "alloc", feature = "std"))]
#[allow(unused_imports)]
#[macro_use]
extern crate alloc;

pub mod error;
pub mod kvn;
pub mod oem;
pub mod opm;
pub mod tdm;

pub use error::CcsdsError;
pub use oem::{parse as parse_oem, write as write_oem, Oem, OemMeta, OemSegment, StateVector};
pub use opm::{parse as parse_opm, Opm};
pub use tdm::{parse as parse_tdm, Tdm, TdmBlock};
