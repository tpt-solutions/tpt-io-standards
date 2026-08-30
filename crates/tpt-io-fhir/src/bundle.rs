//! Bundle traversal and pagination.
//!
//! FHIR Bundles can be large search sets with `link` elements for pagination
//! (`next`, `previous`, `first`, `last`, `self`). This module provides
//! iteration over bundle entries and helpers for following pagination links.

use alloc::string::String;
use alloc::vec::Vec;

use crate::error::FhirError;
use crate::model::{Bundle, BundleEntry, BundleLink, BundleType, Resource};

/// Iterate over the resources in a Bundle, skipping entries without a resource.
pub fn iter_resources(bundle: &Bundle) -> impl Iterator<Item = &Resource> {
    bundle.entry.iter().filter_map(|e| e.resource.as_ref())
}

/// Iterate over entries in a Bundle.
pub fn iter_entries(bundle: &Bundle) -> impl Iterator<Item = &BundleEntry> {
    bundle.entry.iter()
}

/// Filter entries by resource type.
pub fn filter_by_type<'a>(bundle: &'a Bundle, resource_type: &str) -> Vec<&'a Resource> {
    iter_resources(bundle).filter(|r| r.resource_type() == Some(resource_type)).collect()
}

/// Find a pagination link by relation type.
pub fn find_link<'a>(bundle: &'a Bundle, relation: &str) -> Option<&'a BundleLink> {
    bundle.link.iter().find(|l| l.relation == relation)
}

/// The URL of the next page, if present.
pub fn next_page_url(bundle: &Bundle) -> Option<&str> {
    find_link(bundle, "next").map(|l| l.url.as_str())
}

/// The URL of the previous page, if present.
pub fn previous_page_url(bundle: &Bundle) -> Option<&str> {
    find_link(bundle, "previous").map(|l| l.url.as_str())
}

/// The URL of the first page, if present.
pub fn first_page_url(bundle: &Bundle) -> Option<&str> {
    find_link(bundle, "first").map(|l| l.url.as_str())
}

/// The URL of the last page, if present.
pub fn last_page_url(bundle: &Bundle) -> Option<&str> {
    find_link(bundle, "last").map(|l| l.url.as_str())
}

/// The URL of this page (self link), if present.
pub fn self_url(bundle: &Bundle) -> Option<&str> {
    find_link(bundle, "self").map(|l| l.url.as_str())
}

/// Whether this bundle has a next-page link.
pub fn has_next_page(bundle: &Bundle) -> bool {
    next_page_url(bundle).is_some()
}

/// A paginated bundle cursor: holds the current bundle and the base URL for
/// resolving relative pagination links.
#[derive(Debug, Clone)]
pub struct BundleCursor {
    pub bundle: Bundle,
    pub base_url: String,
}

impl BundleCursor {
    pub fn new(bundle: Bundle, base_url: impl Into<String>) -> Self {
        Self { bundle, base_url: base_url.into() }
    }

    /// Total number of entries in the current bundle.
    pub fn entry_count(&self) -> usize {
        self.bundle.entry.len()
    }

    /// Total count reported by the bundle (for search sets), if present.
    pub fn total(&self) -> Option<i64> {
        self.bundle.total
    }

    /// The bundle type.
    pub fn bundle_type(&self) -> Option<BundleType> {
        self.bundle.r#type.clone()
    }

    /// Whether there is a next page.
    pub fn has_next(&self) -> bool {
        has_next_page(&self.bundle)
    }

    /// The resolved next-page URL (relative links resolved against `base_url`).
    pub fn next_url(&self) -> Option<String> {
        next_page_url(&self.bundle).map(|url| self.resolve(url))
    }

    /// Resources in the current bundle.
    pub fn resources(&self) -> impl Iterator<Item = &Resource> {
        iter_resources(&self.bundle)
    }

    /// Resolve a possibly-relative URL against the base URL.
    fn resolve(&self, url: &str) -> String {
        if url.starts_with("http://") || url.starts_with("https://") {
            url.to_string()
        } else if url.starts_with('/') {
            // Extract scheme+host from base.
            if let Some(pos) = self.base_url.find("://") {
                let rest = &self.base_url[pos + 3..];
                if let Some(host_end) = rest.find('/') {
                    format!("{}{}", &self.base_url[..pos + 3 + host_end], url)
                } else {
                    format!("{}{}", self.base_url, url)
                }
            } else {
                url.to_string()
            }
        } else {
            format!("{}/{}", self.base_url.trim_end_matches('/'), url)
        }
    }
}

/// Flatten a bundle into a list of resources, discarding entry metadata.
pub fn flatten(bundle: &Bundle) -> Vec<&Resource> {
    iter_resources(bundle).collect()
}

/// Count resources by type in a bundle.
pub fn count_by_type(bundle: &Bundle) -> Vec<(String, usize)> {
    let mut counts: Vec<(String, usize)> = Vec::new();
    for res in iter_resources(bundle) {
        let rt = res.resource_type().unwrap_or("Unknown").to_string();
        if let Some(entry) = counts.iter_mut().find(|(t, _)| t == &rt) {
            entry.1 += 1;
        } else {
            counts.push((rt, 1));
        }
    }
    counts
}

/// Parse a Bundle from JSON and wrap it in a cursor.
pub fn cursor_from_json(
    json: &str,
    base_url: impl Into<String>,
) -> Result<BundleCursor, FhirError> {
    let bundle = crate::json::parse_bundle(json)?;
    Ok(BundleCursor::new(bundle, base_url))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_bundle() -> Bundle {
        Bundle {
            resource_type: "Bundle".to_string(),
            r#type: Some(BundleType::Searchset),
            total: Some(100),
            link: vec![
                BundleLink {
                    relation: "self".to_string(),
                    url: "https://example.com/fhir/Patient?_page=1".to_string(),
                },
                BundleLink {
                    relation: "next".to_string(),
                    url: "https://example.com/fhir/Patient?_page=2".to_string(),
                },
                BundleLink {
                    relation: "last".to_string(),
                    url: "https://example.com/fhir/Patient?_page=10".to_string(),
                },
            ],
            entry: vec![
                BundleEntry {
                    full_url: Some("Patient/1".to_string()),
                    resource: Some(Resource::Patient(crate::model::Patient {
                        resource_type: "Patient".to_string(),
                        id: Some("1".to_string()),
                        ..Default::default()
                    })),
                    ..Default::default()
                },
                BundleEntry {
                    full_url: Some("Patient/2".to_string()),
                    resource: Some(Resource::Patient(crate::model::Patient {
                        resource_type: "Patient".to_string(),
                        id: Some("2".to_string()),
                        ..Default::default()
                    })),
                    ..Default::default()
                },
            ],
            ..Default::default()
        }
    }

    #[test]
    fn iter_resources_test() {
        let bundle = make_bundle();
        let resources: Vec<_> = iter_resources(&bundle).collect();
        assert_eq!(resources.len(), 2);
    }

    #[test]
    fn pagination_links() {
        let bundle = make_bundle();
        assert!(has_next_page(&bundle));
        assert_eq!(next_page_url(&bundle), Some("https://example.com/fhir/Patient?_page=2"));
        assert_eq!(self_url(&bundle), Some("https://example.com/fhir/Patient?_page=1"));
        assert_eq!(last_page_url(&bundle), Some("https://example.com/fhir/Patient?_page=10"));
        assert!(previous_page_url(&bundle).is_none());
    }

    #[test]
    fn filter_by_type_test() {
        let bundle = make_bundle();
        let patients = filter_by_type(&bundle, "Patient");
        assert_eq!(patients.len(), 2);
    }

    #[test]
    fn count_by_type_test() {
        let bundle = make_bundle();
        let counts = count_by_type(&bundle);
        assert_eq!(counts.len(), 1);
        assert_eq!(counts[0], ("Patient".to_string(), 2));
    }

    #[test]
    fn cursor() {
        let bundle = make_bundle();
        let cursor = BundleCursor::new(bundle, "https://example.com/fhir");
        assert_eq!(cursor.entry_count(), 2);
        assert_eq!(cursor.total(), Some(100));
        assert!(cursor.has_next());
        assert_eq!(cursor.next_url(), Some("https://example.com/fhir/Patient?_page=2".to_string()));
    }

    #[test]
    fn cursor_relative_url() {
        let bundle = Bundle {
            resource_type: "Bundle".to_string(),
            link: vec![BundleLink {
                relation: "next".to_string(),
                url: "Patient?_page=2".to_string(),
            }],
            ..Default::default()
        };
        let cursor = BundleCursor::new(bundle, "https://example.com/fhir");
        assert_eq!(cursor.next_url(), Some("https://example.com/fhir/Patient?_page=2".to_string()));
    }

    #[test]
    fn flatten_test() {
        let bundle = make_bundle();
        let flat = flatten(&bundle);
        assert_eq!(flat.len(), 2);
    }
}
