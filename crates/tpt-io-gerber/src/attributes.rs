//! Gerber X2 / X3 attribute handling (`%TF…%`, `%TA…%`, `%TO…%`).
//!
//! Attributes carry metadata: file-level attributes (function, polarity, part
//! information), aperture attributes (e.g. pad function), and object attributes
//! used for the X3 net-list / component-designation / pin attribution that the
//! `todo.md` checklist calls out as "extended codes".

use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;

/// An ordered, name-keyed collection of Gerber attribute values.
///
/// Attribute names keep their X2 prefixes (`N`, `C`, `P`, `FileFunction`,
/// `AperFunction`, `DR`, …) so callers can introspect both standard and
/// custom attributes. Typed accessors are provided for the common ones.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Attributes {
    /// `(name, values)` pairs, preserving declaration order.
    pub entries: Vec<(String, Vec<String>)>,
}

impl Attributes {
    /// Create an empty attribute set.
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert an attribute, replacing any existing entry with the same name.
    pub fn insert(&mut self, name: impl Into<String>, values: Vec<String>) {
        let name = name.into();
        if let Some(slot) = self.entries.iter_mut().find(|(n, _)| *n == name) {
            slot.1 = values;
            return;
        }
        self.entries.push((name, values));
    }

    /// All values for `name`, or `None` if the attribute is absent.
    pub fn get(&self, name: &str) -> Option<&[String]> {
        self.entries.iter().find(|(n, _)| n == name).map(|(_, v)| v.as_slice())
    }

    /// Whether any attribute is present.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Net names attached via `%TO.N,…%` (X3 net-list attribution).
    pub fn net_names(&self) -> Vec<&str> {
        self.get("N").map(|v| v.iter().map(|s| s.as_str()).collect()).unwrap_or_default()
    }

    /// Component reference designator via `%TO.C,…%` (X3 component designation).
    pub fn component_refdes(&self) -> Option<&str> {
        self.get("C").and_then(|v| v.first()).map(|s| s.as_str())
    }

    /// Pin names attached via `%TO.P,…%` (X3 pin attribution).
    pub fn pin_names(&self) -> Vec<&str> {
        self.get("P").map(|v| v.iter().map(|s| s.as_str()).collect()).unwrap_or_default()
    }

    /// File function via `%TF.FileFunction,…%`.
    pub fn file_function(&self) -> Option<&str> {
        self.get("FileFunction").and_then(|v| v.first()).map(|s| s.as_str())
    }

    /// File polarity via `%TF.FilePolarity,…%` (`Positive` / `Negative`).
    pub fn polarity(&self) -> Option<&str> {
        self.get("FilePolarity").and_then(|v| v.first()).map(|s| s.as_str())
    }

    /// Design-rule metadata via `%TF.DR,…%` or `%TO.DR,…%`.
    pub fn design_rules(&self) -> Vec<&str> {
        let mut out: Vec<&str> = Vec::new();
        for (name, values) in &self.entries {
            if name == "DR" {
                out.extend(values.iter().map(|s| s.as_str()));
            }
        }
        out
    }
}

impl fmt::Display for Attributes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, (name, values)) in self.entries.iter().enumerate() {
            if i > 0 {
                write!(f, "; ")?;
            }
            write!(f, "{name}=")?;
            for (j, v) in values.iter().enumerate() {
                if j > 0 {
                    write!(f, ",")?;
                }
                write!(f, "{v}")?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn net_and_component_accessors() {
        let mut a = Attributes::new();
        a.insert("N", alloc::vec!["net1".into(), "net2".into()]);
        a.insert("C", alloc::vec!["U1".into()]);
        a.insert("P", alloc::vec!["1".into(), "2".into()]);
        assert_eq!(a.net_names(), vec!["net1", "net2"]);
        assert_eq!(a.component_refdes(), Some("U1"));
        assert_eq!(a.pin_names(), vec!["1", "2"]);
    }

    #[test]
    fn insert_replaces() {
        let mut a = Attributes::new();
        a.insert("N", alloc::vec!["a".into()]);
        a.insert("N", alloc::vec!["b".into()]);
        assert_eq!(a.get("N"), Some(&["b".into()][..]));
    }
}
