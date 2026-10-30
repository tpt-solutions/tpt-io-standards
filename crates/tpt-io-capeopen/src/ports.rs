//! CAPE-OPEN unit-operation port mapping.
//!
//! A CAPE-OPEN unit operation exposes material [`Port`]s, each with a
//! [`PortDirection`] (inlet or outlet) and optionally a connected [`Stream`].
//! This module models that topology independent of COM: it is the pure-Rust
//! building block the flowsheet validator ([`crate::flowsheet`]) and the COM
//! unit-operation wrapper build on.

use crate::error::CapeOpenError;

use alloc::string::{String, ToString};
use alloc::vec::Vec;

/// The direction of a material port on a unit operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PortDirection {
    /// Material flows into the unit operation.
    Inlet,
    /// Material flows out of the unit operation.
    Outlet,
}

impl PortDirection {
    /// Whether this is an inlet.
    pub fn is_inlet(&self) -> bool {
        matches!(self, PortDirection::Inlet)
    }

    /// Whether this is an outlet.
    pub fn is_outlet(&self) -> bool {
        matches!(self, PortDirection::Outlet)
    }
}

/// A material port on a unit operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Port {
    /// Port name (unique within its unit operation).
    pub name: String,
    /// Flow direction.
    pub direction: PortDirection,
    /// The id of the stream connected to this port, if any.
    pub connected_stream: Option<String>,
}

impl Port {
    /// Create a disconnected port.
    pub fn new(name: impl Into<String>, direction: PortDirection) -> Self {
        Port { name: name.into(), direction, connected_stream: None }
    }

    /// Whether the port currently has a connected stream.
    pub fn is_connected(&self) -> bool {
        self.connected_stream.is_some()
    }

    /// Connect this port to `stream`, replacing any existing connection.
    ///
    /// Returns an error if `stream` is empty.
    pub fn connect(&mut self, stream: &str) -> Result<(), CapeOpenError> {
        if stream.is_empty() {
            return Err(CapeOpenError::Flowsheet("cannot connect to an empty stream id".into()));
        }
        self.connected_stream = Some(stream.to_string());
        Ok(())
    }

    /// Disconnect this port.
    pub fn disconnect(&mut self) {
        self.connected_stream = None;
    }
}

/// A material stream in a flowsheet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stream {
    /// Stream id (unique within its flowsheet).
    pub id: String,
}

impl Stream {
    /// Create a stream with the given id.
    pub fn new(id: impl Into<String>) -> Self {
        Stream { id: id.into() }
    }
}

/// A CAPE-OPEN unit operation: a named node with inlet/outlet material ports.
#[derive(Debug, Clone, PartialEq)]
pub struct UnitOperation {
    /// Unit operation name (unique within its flowsheet).
    pub name: String,
    /// Ports (inlets and outlets) in declaration order.
    pub ports: Vec<Port>,
}

impl UnitOperation {
    /// Create a unit operation with no ports.
    pub fn new(name: impl Into<String>) -> Self {
        UnitOperation { name: name.into(), ports: Vec::new() }
    }

    /// Add a port. Duplicate port names are rejected later by
    /// [`UnitOperation::validate`] (and by flowsheet validation), so callers may
    /// add freely.
    pub fn add_port(&mut self, port: Port) {
        self.ports.push(port);
    }

    /// Find a port by name.
    pub fn port(&self, name: &str) -> Option<&Port> {
        self.ports.iter().find(|p| p.name == name)
    }

    /// Find a port by name (mutable).
    pub fn port_mut(&mut self, name: &str) -> Option<&mut Port> {
        self.ports.iter_mut().find(|p| p.name == name)
    }

    /// All inlet ports.
    pub fn inlets(&self) -> impl Iterator<Item = &Port> {
        self.ports.iter().filter(|p| p.direction.is_inlet())
    }

    /// All outlet ports.
    pub fn outlets(&self) -> impl Iterator<Item = &Port> {
        self.ports.iter().filter(|p| p.direction.is_outlet())
    }

    /// Find an inlet port by name.
    pub fn inlet(&self, name: &str) -> Option<&Port> {
        self.inlets().find(|p| p.name == name)
    }

    /// Find an outlet port by name.
    pub fn outlet(&self, name: &str) -> Option<&Port> {
        self.outlets().find(|p| p.name == name)
    }

    /// Connect one of this unit operation's ports to a stream.
    pub fn connect(&mut self, port: &str, stream: &str) -> Result<(), CapeOpenError> {
        match self.port_mut(port) {
            Some(p) => p.connect(stream),
            None => Err(CapeOpenError::Flowsheet(format!(
                "unit operation '{}' has no port '{}'",
                self.name, port
            ))),
        }
    }

    /// Validate the unit operation in isolation: unique port names and (by
    /// default) every inlet connected. `require_connected_inlets` lets a
    /// partially-built operation skip the connectivity check.
    pub fn validate(&self, require_connected_inlets: bool) -> Result<(), CapeOpenError> {
        for (i, p) in self.ports.iter().enumerate() {
            if p.name.is_empty() {
                return Err(CapeOpenError::Flowsheet(format!(
                    "unit operation '{}' has a port with an empty name",
                    self.name
                )));
            }
            if self.ports[..i].iter().any(|q| q.name == p.name) {
                return Err(CapeOpenError::Flowsheet(format!(
                    "unit operation '{}' has duplicate port '{}'",
                    self.name, p.name
                )));
            }
        }
        if require_connected_inlets {
            for p in self.inlets() {
                if !p.is_connected() {
                    return Err(CapeOpenError::Flowsheet(format!(
                        "unit operation '{}' inlet '{}' is not connected",
                        self.name, p.name
                    )));
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn connect_unknown_port_errors() {
        let mut uo = UnitOperation::new("Reactor");
        uo.add_port(Port::new("IN", PortDirection::Inlet));
        assert!(uo.connect("NOPE", "S1").is_err());
        assert!(uo.connect("IN", "S1").is_ok());
        assert_eq!(uo.inlet("IN").unwrap().connected_stream.as_deref(), Some("S1"));
    }

    #[test]
    fn duplicate_port_names_rejected() {
        let mut uo = UnitOperation::new("X");
        uo.add_port(Port::new("P", PortDirection::Inlet));
        uo.add_port(Port::new("P", PortDirection::Outlet));
        assert!(uo.validate(false).is_err());
    }
}
