//! OPC UA NodeSet2 XML (`UANodeSet`): reader for the address-space nodes and their references.
//!
//! A NodeSet file lists the namespace URIs it uses, optional aliases for common node ids, and the
//! nodes of an information model: objects, variables, methods, and the type definitions they
//! instantiate. Each node carries its `NodeId`, `BrowseName` and references to other nodes. This
//! crate reads those into a [`NodeSet`].
//!
//! ```
//! let xml = br#"<UANodeSet xmlns="http://opcfoundation.org/UA/2011/03/UANodeSet.xsd">
//!   <NamespaceUris><Uri>http://example.org/plant</Uri></NamespaceUris>
//!   <UAObject NodeId="ns=1;i=1000" BrowseName="1:Plant">
//!     <DisplayName>Plant</DisplayName>
//!     <References><Reference ReferenceType="Organizes" IsForward="false">i=85</Reference></References>
//!   </UAObject>
//! </UANodeSet>"#;
//! let set = tpt_io_opcua::parse(xml).unwrap();
//! assert_eq!(set.nodes[0].browse_name, "1:Plant");
//! set.validate().unwrap();
//! ```
//!
//! Read only. Values, data type encodings and the `Extensions` element are not decoded.
//!
//! Spec access: OPC UA is published by the OPC Foundation. The NodeSet schema is available under
//! the OPC Foundation's terms; no schema text is copied into this crate.

#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]

#[cfg(any(feature = "alloc", feature = "std"))]
#[allow(unused_imports)]
#[macro_use]
extern crate alloc;

pub mod error;
pub mod model;
pub mod reader;

pub use error::OpcUaError;
pub use model::{Node, NodeKind, NodeSet, Reference};
pub use reader::parse;
