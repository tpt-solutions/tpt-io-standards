//! GNSS data: RINEX 3 observation files and SP3 precise orbit files (read only).
//!
//! RINEX 3 observation files list their observation types per satellite system in the header, then give
//! epochs: a `>` line with the time and the satellite count, followed by one line per satellite with fixed
//! 16-character observation fields (14 characters of value, then loss-of-lock and signal-strength flags).
//! SP3 files give satellite positions in kilometres and clock offsets in microseconds at each epoch.
//!
//! ```
//! let sp3 = b"#cP2024 01 01 00 00 00.00000000   1 ORBIT IGS20 FIT  IGS\n*  2024 01 01 00 00 00.00000000\nPG01  1.0 2.0 3.0  4.0\n";
//! let orbits = tpt_io_gnss::parse_sp3(sp3).unwrap();
//! assert_eq!(orbits.epochs[0].positions[0].position_km, [1.0, 2.0, 3.0]);
//! ```
//!
//! Spec access: RINEX is maintained by the IGS and the RINEX working group, and SP3 by the IGS; both are
//! published openly. No text is copied into this crate.

#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]

#[cfg(any(feature = "alloc", feature = "std"))]
#[allow(unused_imports)]
#[macro_use]
extern crate alloc;

pub mod error;
pub mod rinex;
pub mod sp3;

pub use error::GnssError;
pub use rinex::{parse_rinex_obs, Epoch, RinexObs, Satellite};
pub use sp3::{parse_sp3, Sp3, Sp3Epoch, Sp3Position};
