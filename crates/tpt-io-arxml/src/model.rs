//! The ARXML package model.

use alloc::string::{String, ToString};
use alloc::vec::Vec;

use crate::error::ArxmlError;

/// A model element inside a package: its XML tag (kind), short name and UUID.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArxmlElement {
    /// The element's XML tag, e.g. `APPLICATION-SW-COMPONENT-TYPE`.
    pub kind: String,
    /// The element's `SHORT-NAME`.
    pub short_name: String,
    /// The `UUID` attribute, when present.
    pub uuid: Option<String>,
}

/// An `AR-PACKAGE`: its path, elements and nested packages.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Package {
    /// The package's `SHORT-NAME`.
    pub short_name: String,
    /// Absolute path from the root, e.g. `/Components/Sensors`.
    pub path: String,
    /// The `UUID` attribute, when present.
    pub uuid: Option<String>,
    /// Elements in document order.
    pub elements: Vec<ArxmlElement>,
    /// Nested packages in document order.
    pub packages: Vec<Package>,
}

/// A whole ARXML file.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ArxmlFile {
    /// The schema release from the default namespace, e.g. `r4.0`, when it can be read.
    pub schema_version: Option<String>,
    /// Top-level packages in document order.
    pub packages: Vec<Package>,
}

impl ArxmlFile {
    /// Every package in the file, depth first, in document order.
    pub fn all_packages(&self) -> Vec<&Package> {
        fn walk<'a>(p: &'a Package, out: &mut Vec<&'a Package>) {
            out.push(p);
            for c in &p.packages {
                walk(c, out);
            }
        }
        let mut out = Vec::new();
        for p in &self.packages {
            walk(p, &mut out);
        }
        out
    }

    /// Check that short names are unique among siblings: packages within a parent, and elements
    /// within a package.
    pub fn validate(&self) -> Result<(), ArxmlError> {
        fn check(packages: &[Package]) -> Result<(), ArxmlError> {
            let mut names: Vec<&str> = packages.iter().map(|p| p.short_name.as_str()).collect();
            names.sort_unstable();
            if names.windows(2).any(|w| w[0] == w[1]) {
                return Err(ArxmlError::Invalid("duplicate package short name".to_string()));
            }
            for p in packages {
                let mut elems: Vec<&str> =
                    p.elements.iter().map(|e| e.short_name.as_str()).collect();
                elems.sort_unstable();
                if elems.windows(2).any(|w| w[0] == w[1]) {
                    return Err(ArxmlError::Invalid(alloc::format!(
                        "duplicate element short name in {}",
                        p.path
                    )));
                }
                check(&p.packages)?;
            }
            Ok(())
        }
        check(&self.packages)
    }
}
