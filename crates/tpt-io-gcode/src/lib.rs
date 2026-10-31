//! G-code programs for CNC mills and 3D printers: reader, writer and motion summary.
//!
//! A program is a list of blocks, one per line. A block holds words (a letter and a number, such
//! as `G1`, `X10.5` or `F300`), an optional line number (`N10`) and an optional comment. Two
//! comment styles are read: `;` to the end of the line (RepRap and most 3D-printer firmware) and
//! `(…)` (LinuxCNC and Fanuc-style controllers). Each word keeps its source text so a round trip
//! writes back what was read.
//!
//! ```
//! let program = b"G21 ; millimetres\nG90\nG1 X10 Y0 F600\nG1 X10 Y10\n";
//! let p = tpt_io_gcode::parse(program).unwrap();
//! let summary = p.motion_summary();
//! assert!((summary.path_length_mm - 20.0).abs() < 1e-9);
//! ```
//!
//! [`Program::motion_summary`] tracks the modal state of the common words: absolute and
//! incremental positioning (`G90`/`G91`), units (`G20`/`G21`), and the motion mode (`G0`/`G1`,
//! `G2`/`G3`). It reports the path length of linear moves in millimetres, the number of rapid,
//! linear and arc moves, and the bounding box of the XY positions reached.
//!
//! Spec access: G-code dialects are documented by the controller vendors (LinuxCNC and RepRap
//! are open source). No vendor text is copied here; the word syntax follows the common RS-274
//! conventions.

#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]

#[cfg(any(feature = "alloc", feature = "std"))]
#[allow(unused_imports)]
#[macro_use]
extern crate alloc;

pub mod error;
pub mod program;

pub use error::GcodeError;
pub use program::{parse, write, Block, Comment, Dialect, MotionSummary, Program, Word};
