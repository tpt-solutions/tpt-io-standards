//! GridLAB-D model files (`.glm`): reader, writer and object validation.
//!
//! A GLM file is a list of top-level statements: `clock` and `module` blocks, `class`
//! definitions, `#` directives such as `#set` or `#include`, and `object` blocks.
//! Objects nest (a `meter` inside a `triplex_meter`, for example) and carry
//! `name value;` properties whose values are kept as the raw text in the file.
//!
//! ```
//! let src = b"clock { timezone EST+5EDT; }\n\
//! object node {\n  name n1;\n  nominal_voltage 2400;\n}\n";
//! let glm = tpt_io_gridlabd::parse(src).unwrap();
//! let node = glm.objects_of_class("node").next().unwrap();
//! assert_eq!(node.name(), Some("n1"));
//! assert_eq!(node.prop("nominal_voltage"), Some("2400"));
//! ```
//!
//! Values are not evaluated: `100+50j` or `'2000-01-01 0:00:00'` come back exactly as
//! written. The reader does not follow `#include`.
//!
//! Spec access: the GLM syntax is documented in the GridLAB-D user manual, distributed
//! under a BSD-style licence. No manual text is copied into this crate.

#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]

#[cfg(any(feature = "alloc", feature = "std"))]
#[allow(unused_imports)]
#[macro_use]
extern crate alloc;

pub mod error;
pub mod glm;

pub use error::GlmError;
pub use glm::{parse, write, Glm, Item, Object};
