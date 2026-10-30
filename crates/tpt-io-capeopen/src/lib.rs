//! CAPE-OPEN 1.1 / 2.0 interface definitions and a pure-Rust thermodynamic /
//! flowsheet data model for process simulation interoperability.
//!
//! ## Pure-Rust model (always available)
//!
//! [`model`] defines the CAPE-OPEN concepts — [`model::ComponentList`],
//! [`model::Phase`], [`model::BaseProperty`], [`model::PropertyPackage`], and
//! [`model::StreamState`] — with no COM and no Windows dependency. [`parser`]
//! and [`writer`] turn a [`model::PropertyPackage`] into a stable, round-trippable
//! text form and back, implementing the shared [`tpt_io_core::Reader`] /
//! [`tpt_io_core::Writer`] traits. [`ports`] and [`flowsheet`] model unit
//! operation port mapping and validate flowsheet connectivity.
//!
//! ```rust
//! use tpt_io_core::{Reader, Writer};
//! use tpt_io_capeopen::model::{ComponentList, Phase, PropertyPackage};
//! use tpt_io_capeopen::parser::PropertyPackageReader;
//! use tpt_io_capeopen::writer::PropertyPackageWriter;
//!
//! let mut pp = PropertyPackage::new("MyPP", ComponentList::from_ids(&["WATER", "METHANOL"]));
//! pp.supported_phases.push(Phase::Vapor);
//! let bytes = PropertyPackageWriter::new().serialize(&pp).unwrap();
//! let back: PropertyPackage = PropertyPackageReader::new().parse(&bytes).unwrap();
//! assert_eq!(pp, back);
//! ```
//!
//! ## COM interop (Windows-only, `com` feature)
//!
//! [`com`] provides `#[repr(C)]` vtable definitions and safe wrappers around
//! `ICapeThermoMaterial`, `ICapeThermoCalculation`, and `ICapeThermoPropertyPackage`.
//! It is compiled only with `--features com` on Windows and depends on the
//! `windows` crate. **The default (no-`com`) build is pure Rust and cross-platform.**
//!
//! ## Downstream integration
//!
//! `thermo-integration` and `eng-unit-ops-integration` expose stable conversion
//! stubs into the future `tpt-thermo` / `tpt-eng-unit-ops` crates (see
//! [`integration`]).
//!
//! # Safety
//!
//! All `unsafe` in this crate is confined to the `com` module (COM vtable calls).
//! The default, no-`com` build contains no `unsafe` code.

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(any(feature = "alloc", feature = "std"))]
extern crate alloc;

#[cfg(not(any(feature = "alloc", feature = "std")))]
compile_error!("tpt-io-capeopen requires the `alloc` or `std` feature.");

pub mod error;
pub mod flowsheet;
pub mod model;
pub mod parser;
pub mod ports;
pub mod writer;

#[cfg(any(feature = "thermo-integration", feature = "eng-unit-ops-integration"))]
pub mod integration;

#[cfg(all(feature = "com", windows))]
pub mod com;

pub use error::CapeOpenError;
pub use flowsheet::Flowsheet;
pub use model::{
    BaseProperty, CalculationKind, Component, ComponentList, Phase, PropertyCalculationRequest,
    PropertyPackage, StreamState,
};
pub use parser::{parse_property_package, PropertyPackageReader};
pub use ports::{Port, PortDirection, Stream, UnitOperation};
pub use writer::{encode_property_package, serialize, PropertyPackageWriter};

#[cfg(any(feature = "alloc", feature = "std"))]
pub use tpt_io_core::ParseError;
pub use tpt_io_core::{
    Encoding, FormatExtension, NoProgress, Progress, ProgressFn, Reader, Schema,
    ValidationSettings, Validator, Writer,
};
