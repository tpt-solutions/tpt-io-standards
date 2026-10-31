//! AutomationML (CAEX, IEC 62714) files: reader for the instance hierarchy, attributes and links.
//!
//! An AutomationML file is a CAEX document. Its `InstanceHierarchy` holds the plant objects as nested
//! `InternalElement`s, each with a name, an id, a reference to its system-unit class, and `Attribute`s
//! with values. `InternalLink` elements connect objects. This crate reads that structure into an
//! [`Element`] tree and a link list. Role and interface class libraries are not resolved.
//!
//! ```
//! let caex = br#"<CAEXFile><InstanceHierarchy Name="Plant">
//!   <InternalElement Name="Pump1" ID="p1" RefBaseSystemUnitPath="Lib/Pump">
//!     <Attribute Name="Power" AttributeDataType="xs:double"><Value>7.5</Value></Attribute>
//!   </InternalElement>
//!   <InternalElement Name="Tank1" ID="t1"/>
//!   <InternalLink Name="pipe" RefPartnerSideA="p1:out" RefPartnerSideB="t1:in"/>
//! </InstanceHierarchy></CAEXFile>"#;
//! let file = tpt_io_automationml::parse(caex).unwrap();
//! assert_eq!(file.hierarchy[0].attributes[0].value.as_deref(), Some("7.5"));
//! assert_eq!(file.links[0].side_b, "t1:in");
//! ```
//!
//! Read only. Role, interface and system-unit class libraries are kept as paths, not resolved.
//!
//! Spec access: AutomationML is defined by IEC 62714 and the AutomationML consortium. No standard text is
//! copied into this crate.

#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]

#[cfg(any(feature = "alloc", feature = "std"))]
#[allow(unused_imports)]
#[macro_use]
extern crate alloc;

pub mod caex;
pub mod error;

pub use caex::{parse, Attribute, CaexFile, Element, Link};
pub use error::AutomationMlError;
