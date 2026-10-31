//! LandXML model: units, project name, surfaces and alignment names.

use alloc::string::{String, ToString};
use alloc::vec::Vec;

use crate::error::LandXmlError;

/// A TIN surface: named points with ids, and faces as triples of point ids.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Surface {
    /// Surface name.
    pub name: String,
    /// `(id, [x, y, z])` pairs in file order.
    pub points: Vec<(u32, [f64; 3])>,
    /// Faces as three point ids each.
    pub faces: Vec<[u32; 3]>,
}

/// A LandXML document.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct LandXml {
    /// The linear unit (`meter`, `foot`, …) from the metric or imperial unit element.
    pub linear_unit: Option<String>,
    /// `Project/@name`, if present.
    pub project: Option<String>,
    /// TIN surfaces in document order.
    pub surfaces: Vec<Surface>,
    /// Alignment names in document order (their geometry is not read).
    pub alignments: Vec<String>,
}

impl LandXml {
    /// Check that every face refers to three distinct points that exist in its surface.
    pub fn validate(&self) -> Result<(), LandXmlError> {
        for s in &self.surfaces {
            let mut ids: Vec<u32> = s.points.iter().map(|(id, _)| *id).collect();
            ids.sort_unstable();
            if ids.windows(2).any(|w| w[0] == w[1]) {
                return Err(LandXmlError::Invalid(alloc::format!("duplicate point id in surface {}", s.name)));
            }
            for f in &s.faces {
                for id in f {
                    if ids.binary_search(id).is_err() {
                        return Err(LandXmlError::Invalid(alloc::format!("face refers to unknown point {id}")));
                    }
                }
                if f[0] == f[1] || f[1] == f[2] || f[0] == f[2] {
                    return Err(LandXmlError::Invalid("face repeats a point".to_string()));
                }
            }
        }
        Ok(())
    }
}
