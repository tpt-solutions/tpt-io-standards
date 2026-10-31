//! The NodeSet model.

use alloc::string::{String, ToString};
use alloc::vec::Vec;

use crate::error::OpcUaError;

/// The kind of a node, from its XML element name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NodeKind {
    /// `UAObject`.
    Object,
    /// `UAVariable`.
    Variable,
    /// `UAMethod`.
    Method,
    /// `UAObjectType`.
    ObjectType,
    /// `UAVariableType`.
    VariableType,
    /// `UADataType`.
    DataType,
    /// `UAReferenceType`.
    ReferenceType,
    /// `UAView`.
    View,
    /// Any other element, kept by name.
    Other(String),
}

/// A reference from one node to another.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reference {
    /// The reference type, e.g. `HasComponent` or an aliased node id.
    pub reference_type: String,
    /// `IsForward` (defaults to `true`).
    pub is_forward: bool,
    /// The target node id, as written (`ns=1;i=1001`, `i=85`, or an alias).
    pub target: String,
}

/// One node of the address space.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Node {
    /// The node kind.
    pub kind: NodeKind,
    /// The `NodeId`.
    pub node_id: String,
    /// The `BrowseName`, e.g. `1:Plant`.
    pub browse_name: String,
    /// The `DisplayName`, if given.
    pub display_name: Option<String>,
    /// The `DataType` attribute of a variable, if given.
    pub data_type: Option<String>,
    /// References in document order.
    pub references: Vec<Reference>,
}

/// A parsed NodeSet.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct NodeSet {
    /// Namespace URIs in index order (index 0 is the OPC UA base namespace).
    pub namespace_uris: Vec<String>,
    /// `(alias, node id)` pairs from `<Aliases>`.
    pub aliases: Vec<(String, String)>,
    /// Nodes in document order.
    pub nodes: Vec<Node>,
}

impl NodeSet {
    /// Check that node ids are unique and that every node has a browse name.
    pub fn validate(&self) -> Result<(), OpcUaError> {
        let mut ids: Vec<&str> = self.nodes.iter().map(|n| n.node_id.as_str()).collect();
        ids.sort_unstable();
        if ids.windows(2).any(|w| w[0] == w[1]) {
            return Err(OpcUaError::Invalid("duplicate NodeId".to_string()));
        }
        if self.nodes.iter().any(|n| n.browse_name.is_empty()) {
            return Err(OpcUaError::Invalid("node without a BrowseName".to_string()));
        }
        Ok(())
    }
}
