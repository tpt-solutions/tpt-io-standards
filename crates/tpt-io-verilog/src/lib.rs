//! Structural Verilog netlists: reader and writer for modules, ports, nets, instances and `assign`.
//!
//! The reader covers the structural subset that netlists from synthesis tools use: `module` headers
//! with port lists, `input`, `output`, `inout` and `wire` declarations (with optional bit ranges),
//! module instances with named (`.A(net)`) or positional connections, and simple `assign` statements.
//! Behavioural code (`always`, `initial`, expressions) is rejected rather than approximated.
//!
//! ```
//! let src = b"module top (a, y);\n input a;\n output y;\n INV u1 (.A(a), .Y(y));\nendmodule\n";
//! let netlist = tpt_io_verilog::parse(src).unwrap();
//! assert_eq!(netlist.modules[0].instances[0].cell, "INV");
//! ```
//!
//! Spec access: Verilog is standardised as IEEE 1364, which is available under IEEE's terms. No
//! standard text is copied into this crate.

#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]

#[cfg(any(feature = "alloc", feature = "std"))]
#[allow(unused_imports)]
#[macro_use]
extern crate alloc;

pub mod error;
pub mod netlist;

pub use error::VerilogError;
pub use netlist::{parse, write, Connection, Direction, Instance, Module, Netlist, Port};
