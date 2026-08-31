//! Flowsheet topology and stream-connection validation.
//!
//! A CAPE-OPEN flowsheet connects unit operations through material streams: an
//! outlet port produces a stream that feeds one or more inlet ports. This module
//! models that graph in pure Rust and [`Flowsheet::validate`]s the connectivity
//! invariants a CAPE-OPEN simulation environment expects:
//!
//! * every stream is produced by at most one outlet port and consumed by at
//!   most one inlet port (a stream is a 1-to-1 material connection),
//! * every inlet port is connected to a declared stream,
//! * every stream referenced by a port exists in the flowsheet, and
//! * no outlet port feeds more than one stream.
//!
//! Recycle loops (an outlet feeding back to an earlier inlet) are explicitly
//! allowed — they are legal in CAPE-OPEN flowsheets.

use crate::error::CapeOpenError;
use crate::ports::{PortDirection, Stream, UnitOperation};

use alloc::string::String;
use alloc::vec::Vec;

/// A CAPE-OPEN flowsheet: a set of unit operations joined by material streams.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Flowsheet {
    /// Unit operations in the flowsheet.
    pub unit_operations: Vec<UnitOperation>,
    /// Declared streams in the flowsheet.
    pub streams: Vec<Stream>,
}

impl Flowsheet {
    /// Create an empty flowsheet.
    pub fn new() -> Self {
        Flowsheet { unit_operations: Vec::new(), streams: Vec::new() }
    }

    /// Add a unit operation (no-op if a unit with the same name already exists).
    pub fn add_unit_operation(&mut self, uo: UnitOperation) {
        if !self.unit_operations.iter().any(|u| u.name == uo.name) {
            self.unit_operations.push(uo);
        }
    }

    /// Declare a stream (no-op if a stream with the same id already exists).
    pub fn add_stream(&mut self, stream: Stream) {
        if !self.streams.iter().any(|s| s.id == stream.id) {
            self.streams.push(stream);
        }
    }

    /// Connect a unit operation's port to a stream, declaring the stream if it
    /// does not yet exist.
    pub fn connect(&mut self, unit: &str, port: &str, stream: &str) -> Result<(), CapeOpenError> {
        if stream.is_empty() {
            return Err(CapeOpenError::Flowsheet("cannot connect via an empty stream id".into()));
        }
        let uo = self
            .unit_operations
            .iter_mut()
            .find(|u| u.name == unit)
            .ok_or_else(|| CapeOpenError::Flowsheet(format!("no unit operation '{unit}'")))?;
        uo.connect(port, stream)?;
        self.add_stream(Stream::new(stream));
        Ok(())
    }

    /// Validate flowsheet topology.
    ///
    /// Checks unique unit-operation / stream ids, port existence, and the
    /// stream-connection invariants described on the module. Structural (naming)
    /// problems are reported first, then connectivity problems.
    pub fn validate(&self) -> Result<(), CapeOpenError> {
        // Unique unit-operation names.
        for (i, u) in self.unit_operations.iter().enumerate() {
            if u.name.is_empty() {
                return Err(CapeOpenError::Flowsheet(
                    "flowsheet has a unit operation with an empty name".into(),
                ));
            }
            if self.unit_operations[..i].iter().any(|p| p.name == u.name) {
                return Err(CapeOpenError::Flowsheet(format!(
                    "duplicate unit operation '{}'",
                    u.name
                )));
            }
            u.validate(false)?;
        }

        // Unique stream ids.
        for (i, s) in self.streams.iter().enumerate() {
            if s.id.is_empty() {
                return Err(CapeOpenError::Flowsheet(
                    "flowsheet has a stream with an empty id".into(),
                ));
            }
            if self.streams[..i].iter().any(|p| p.id == s.id) {
                return Err(CapeOpenError::Flowsheet(format!("duplicate stream '{}'", s.id)));
            }
        }

        // Walk every port connection and tally how each stream is used.
        let mut outlet_producers: Vec<&String> = Vec::new();
        let mut inlet_consumers: Vec<&String> = Vec::new();
        for uo in &self.unit_operations {
            for p in &uo.ports {
                let stream = match &p.connected_stream {
                    Some(s) => s,
                    None => {
                        if p.direction == PortDirection::Inlet {
                            return Err(CapeOpenError::Flowsheet(format!(
                                "inlet '{}' on unit '{}' is not connected",
                                p.name, uo.name
                            )));
                        }
                        continue;
                    }
                };
                if !self.streams.iter().any(|s| &s.id == stream) {
                    return Err(CapeOpenError::Flowsheet(format!(
                        "port '{}' on unit '{}' connects to undeclared stream '{stream}'",
                        p.name, uo.name
                    )));
                }
                match p.direction {
                    PortDirection::Inlet => inlet_consumers.push(stream),
                    PortDirection::Outlet => outlet_producers.push(stream),
                }
            }
        }

        // A stream may be produced by at most one outlet and consumed by at most
        // one inlet.
        for s in &self.streams {
            let producer_count =
                outlet_producers.iter().filter(|p| p.as_str() == s.id.as_str()).count();
            let consumer_count =
                inlet_consumers.iter().filter(|p| p.as_str() == s.id.as_str()).count();
            if producer_count > 1 {
                return Err(CapeOpenError::Flowsheet(format!(
                    "stream '{}' is produced by {producer_count} outlets (expected at most 1)",
                    s.id
                )));
            }
            if consumer_count > 1 {
                return Err(CapeOpenError::Flowsheet(format!(
                    "stream '{}' feeds {consumer_count} inlets (expected at most 1)",
                    s.id
                )));
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ports::Port;

    fn mixer() -> UnitOperation {
        let mut uo = UnitOperation::new("Mixer");
        uo.add_port(Port::new("IN1", PortDirection::Inlet));
        uo.add_port(Port::new("IN2", PortDirection::Inlet));
        uo.add_port(Port::new("OUT", PortDirection::Outlet));
        uo
    }

    fn splitter() -> UnitOperation {
        let mut uo = UnitOperation::new("Splitter");
        uo.add_port(Port::new("IN", PortDirection::Inlet));
        uo.add_port(Port::new("OUT", PortDirection::Outlet));
        uo
    }

    #[test]
    fn valid_linear_flowsheet_passes() {
        let mut fs = Flowsheet::new();
        fs.add_unit_operation(mixer());
        fs.add_unit_operation(splitter());
        fs.connect("Mixer", "OUT", "S1").unwrap();
        fs.connect("Splitter", "IN", "S1").unwrap();
        fs.connect("Mixer", "IN1", "FEED1").unwrap();
        fs.connect("Mixer", "IN2", "FEED2").unwrap();
        assert!(fs.validate().is_ok());
    }

    #[test]
    fn dangling_inlet_fails() {
        let mut fs = Flowsheet::new();
        fs.add_unit_operation(mixer());
        assert!(fs.validate().is_err());
    }

    #[test]
    fn one_stream_two_inlets_fails() {
        let mut fs = Flowsheet::new();
        fs.add_unit_operation(mixer());
        fs.connect("Mixer", "IN1", "S1").unwrap();
        fs.connect("Mixer", "IN2", "S1").unwrap();
        assert!(fs.validate().is_err());
    }

    #[test]
    fn undeclared_stream_ref_fails() {
        let mut fs = Flowsheet::new();
        fs.add_unit_operation(mixer());
        fs.unit_operations[0].ports[2].connect("GHOST").unwrap();
        assert!(fs.validate().is_err());
    }
}
