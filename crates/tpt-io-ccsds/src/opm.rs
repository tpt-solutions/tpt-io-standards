//! Orbit Parameter Message (OPM): one state vector with header and metadata key/values. Read only.

use alloc::string::{String, ToString};
use alloc::vec::Vec;

use crate::error::CcsdsError;
use crate::kvn::{classify, lines, number, Line};
use crate::oem::StateVector;

/// A parsed OPM. Every key/value pair is kept in file order; the state vector is extracted
/// from the `EPOCH`, `X`, `Y`, `Z`, `X_DOT`, `Y_DOT` and `Z_DOT` keys when they are present.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Opm {
    /// `CCSDS_OPM_VERS`, e.g. `2.0`.
    pub version: String,
    /// All key/value pairs in file order, including the version.
    pub fields: Vec<(String, String)>,
    /// The state vector, when all six components are present.
    pub state: Option<StateVector>,
}

impl Opm {
    /// The value of the first pair with `key`.
    pub fn get(&self, key: &str) -> Option<&str> {
        self.fields.iter().find(|(k, _)| k == key).map(|(_, v)| v.as_str())
    }
}

/// Parse an OPM file.
pub fn parse(input: &[u8]) -> Result<Opm, CcsdsError> {
    let mut opm = Opm::default();
    for raw in lines(input)? {
        match classify(raw) {
            Some(Line::Pair(k, v)) => {
                if k == "CCSDS_OPM_VERS" {
                    opm.version = v.to_string();
                }
                opm.fields.push((k.to_string(), v.to_string()));
            }
            Some(Line::Keyword(_)) | Some(Line::Other(_)) | None => {}
        }
    }
    if opm.version.is_empty() {
        return Err(CcsdsError::Invalid("missing CCSDS_OPM_VERS".into()));
    }
    opm.state = state_from(&opm)?;
    Ok(opm)
}

fn state_from(opm: &Opm) -> Result<Option<StateVector>, CcsdsError> {
    let (Some(epoch), Some(x), Some(y), Some(z), Some(xd), Some(yd), Some(zd)) = (
        opm.get("EPOCH"),
        opm.get("X"),
        opm.get("Y"),
        opm.get("Z"),
        opm.get("X_DOT"),
        opm.get("Y_DOT"),
        opm.get("Z_DOT"),
    ) else {
        return Ok(None);
    };
    Ok(Some(StateVector {
        epoch: epoch.to_string(),
        position: [number(x, "X")?, number(y, "Y")?, number(z, "Z")?],
        velocity: [number(xd, "X_DOT")?, number(yd, "Y_DOT")?, number(zd, "Z_DOT")?],
    }))
}
