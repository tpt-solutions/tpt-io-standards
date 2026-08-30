//! Spatial indexing with an R-tree (Sort-Tile-Recursive bulk load).
//!
//! The [`SpatialIndex`] accelerates the three classic spatial queries called out
//! in the workspace spec:
//!
//! * **range query** — every item whose bounding box intersects a query box,
//! * **nearest neighbour** — the item whose box is closest to a query point,
//! * **point-in-polygon candidate selection** — combine [`SpatialIndex::range`]
//!   with the ray-casting [`point_in_ring`] test to find which polygons contain
//!   a point without scanning every polygon.
//!
//! The tree is static (built once); it is intended for the read/query phase of
//! large imported datasets.

use alloc::vec::Vec;

use crate::error::GeoError;

/// An axis-aligned bounding box `[xmin, ymin, xmax, ymax]`.
pub type Bbox = [f64; 4];

/// Types that expose a bounding box for indexing.
pub trait HasBounds {
    /// The bounding box of this item.
    fn bounds(&self) -> Bbox;
}

/// Default capacity of a leaf / node.
const MAX_ENTRIES: usize = 16;

/// An indexed entry (leaf).
#[derive(Debug, Clone)]
struct Leaf<T> {
    bbox: Bbox,
    item: T,
}

#[derive(Debug, Clone)]
enum Node<T> {
    Leaf(Vec<Leaf<T>>),
    Internal { bbox: Bbox, children: Vec<Node<T>> },
}

fn bbox_union(a: Bbox, b: Bbox) -> Bbox {
    [a[0].min(b[0]), a[1].min(b[1]), a[2].max(b[2]), a[3].max(b[3])]
}

fn bbox_intersects(a: &Bbox, b: &Bbox) -> bool {
    a[0] <= b[2] && a[2] >= b[0] && a[1] <= b[3] && a[3] >= b[1]
}

/// A static R-tree over items that implement [`HasBounds`].
#[derive(Debug, Clone)]
pub struct SpatialIndex<T: HasBounds> {
    root: Option<Node<T>>,
}

impl<T: HasBounds> Default for SpatialIndex<T> {
    fn default() -> Self {
        Self { root: None }
    }
}

impl<T: HasBounds> SpatialIndex<T> {
    /// Build an R-tree from a list of items via STR bulk loading.
    pub fn build(items: Vec<T>) -> Self {
        if items.is_empty() {
            return Self { root: None };
        }
        let leaves: Vec<Leaf<T>> = items
            .into_iter()
            .map(|item| {
                let bbox = item.bounds();
                Leaf { bbox, item }
            })
            .collect();
        let root = build_str(leaves);
        Self { root: Some(root) }
    }

    /// Number of indexed items.
    pub fn len(&self) -> usize {
        match &self.root {
            Some(Node::Leaf(leaves)) => leaves.len(),
            Some(Node::Internal { .. }) => count_leaves(self.root.as_ref().unwrap()),
            None => 0,
        }
    }

    /// Whether the index is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Return all items whose bounding box intersects `query`.
    pub fn range(&self, query: &Bbox) -> Vec<&T> {
        let mut out = Vec::new();
        if let Some(root) = &self.root {
            collect_range(root, query, &mut out);
        }
        out
    }

    /// Return the item whose box is nearest to `(x, y)`.
    ///
    /// Distance is measured to the nearest point of each item's bounding box;
    /// for point items this is the Euclidean distance.
    pub fn nearest(&self, x: f64, y: f64) -> Option<&T> {
        let mut best: Option<(&T, f64)> = None;
        if let Some(root) = &self.root {
            nearest_walk(root, x, y, &mut best);
        }
        best.map(|(t, _)| t)
    }

    /// Candidate items whose box contains the point `(x, y)`.
    ///
    /// This is a super-set of the items that *actually* contain the point; callers
    /// should refine with [`point_in_ring`] (for polygons) or an exact test.
    pub fn candidates_at_point(&self, x: f64, y: f64) -> Vec<&T> {
        let q: Bbox = [x, y, x, y];
        self.range(&q)
    }
}

fn count_leaves<T: HasBounds>(node: &Node<T>) -> usize {
    match node {
        Node::Leaf(leaves) => leaves.len(),
        Node::Internal { children, .. } => children.iter().map(count_leaves).sum(),
    }
}

fn build_str<T: HasBounds>(mut leaves: Vec<Leaf<T>>) -> Node<T> {
    if leaves.len() <= MAX_ENTRIES {
        return Node::Leaf(leaves);
    }
    // Sort-Tile-Recursive: partition leaves into a grid of S×S tiles.
    let p = leaves.len().div_ceil(MAX_ENTRIES); // number of leaf nodes
    let s = (p as f64).sqrt().ceil() as usize; // tiles per axis
    let slice_size = s * MAX_ENTRIES;
    leaves.sort_by(|a, b| center_x(&a.bbox).partial_cmp(&center_x(&b.bbox)).unwrap());
    let mut nodes: Vec<Node<T>> = Vec::with_capacity(p);
    let i = 0;
    while i < leaves.len() {
        let end = core::cmp::min(i + slice_size, leaves.len());
        // `drain` shifts the remaining items down to index `i`, so the next
        // vertical slice also starts at `i`.
        let mut chunk: Vec<Leaf<T>> = leaves.drain(i..end).collect();
        chunk.sort_by(|a, b| center_y(&a.bbox).partial_cmp(&center_y(&b.bbox)).unwrap());
        let mut j = 0;
        while j < chunk.len() {
            let e = core::cmp::min(j + MAX_ENTRIES, chunk.len());
            let group: Vec<Leaf<T>> = chunk.drain(j..e).collect();
            j = e;
            nodes.push(Node::Leaf(group));
        }
    }
    build_internal(nodes)
}

fn build_internal<T: HasBounds>(mut nodes: Vec<Node<T>>) -> Node<T> {
    if nodes.len() == 1 {
        return nodes.pop().unwrap();
    }
    if nodes.len() <= MAX_ENTRIES {
        let bbox = nodes
            .iter()
            .map(node_bbox)
            .fold([f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY], bbox_union);
        return Node::Internal { bbox, children: nodes };
    }
    // Treat child nodes as entries and STR-partition them.
    let p = nodes.len().div_ceil(MAX_ENTRIES);
    let s = (p as f64).sqrt().ceil() as usize;
    nodes.sort_by(|a, b| center_x(&node_bbox(a)).partial_cmp(&center_x(&node_bbox(b))).unwrap());
    let slice_size = s * MAX_ENTRIES;
    let mut children_groups: Vec<Node<T>> = Vec::new();
    let mut i = 0;
    while i < nodes.len() {
        let end = core::cmp::min(i + slice_size, nodes.len());
        let mut group = Vec::new();
        for n in nodes.drain(i..end) {
            group.push(n);
        }
        group
            .sort_by(|a, b| center_y(&node_bbox(a)).partial_cmp(&center_y(&node_bbox(b))).unwrap());
        let mut j = 0;
        while j < group.len() {
            let e = core::cmp::min(j + MAX_ENTRIES, group.len());
            let sub: Vec<Node<T>> = group.drain(j..e).collect();
            j = e;
            let bbox = sub.iter().map(node_bbox).fold(
                [f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY],
                bbox_union,
            );
            children_groups.push(Node::Internal { bbox, children: sub });
        }
        i = 0;
    }
    build_internal(children_groups)
}

fn node_bbox<T: HasBounds>(node: &Node<T>) -> Bbox {
    match node {
        Node::Leaf(leaves) => leaves
            .iter()
            .map(|l| l.bbox)
            .fold([f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY], bbox_union),
        Node::Internal { bbox, .. } => *bbox,
    }
}

fn center_x(b: &Bbox) -> f64 {
    (b[0] + b[2]) / 2.0
}
fn center_y(b: &Bbox) -> f64 {
    (b[1] + b[3]) / 2.0
}

fn collect_range<'a, T: HasBounds>(node: &'a Node<T>, query: &Bbox, out: &mut Vec<&'a T>) {
    match node {
        Node::Leaf(leaves) => {
            for l in leaves {
                if bbox_intersects(&l.bbox, query) {
                    out.push(&l.item);
                }
            }
        }
        Node::Internal { bbox, children } => {
            if bbox_intersects(bbox, query) {
                for c in children {
                    collect_range(c, query, out);
                }
            }
        }
    }
}

fn point_to_bbox_dist(x: f64, y: f64, b: &Bbox) -> f64 {
    let dx = if x < b[0] {
        b[0] - x
    } else if x > b[2] {
        x - b[2]
    } else {
        0.0
    };
    let dy = if y < b[1] {
        b[1] - y
    } else if y > b[3] {
        y - b[3]
    } else {
        0.0
    };
    (dx * dx + dy * dy).sqrt()
}

fn nearest_walk<'a, T: HasBounds>(
    node: &'a Node<T>,
    x: f64,
    y: f64,
    best: &mut Option<(&'a T, f64)>,
) {
    match node {
        Node::Leaf(leaves) => {
            for l in leaves {
                let d = point_to_bbox_dist(x, y, &l.bbox);
                if best.as_ref().map(|(_, bd)| d < *bd).unwrap_or(true) {
                    *best = Some((&l.item, d));
                }
            }
        }
        Node::Internal { bbox, children } => {
            // Prune: if the closest point of this node is farther than best, skip.
            let d = point_to_bbox_dist(x, y, bbox);
            if best.as_ref().map(|(_, bd)| d > *bd).unwrap_or(false) {
                return;
            }
            for c in children {
                nearest_walk(c, x, y, best);
            }
        }
    }
}

/// Ray-casting point-in-polygon test for a single ring (no holes).
///
/// `ring` is an ordered list of `(x, y)` vertices; the test works regardless of
/// whether the ring is closed (first == last). Returns `true` when `(px, py)`
/// lies inside the ring.
pub fn point_in_ring(ring: &[(f64, f64)], px: f64, py: f64) -> bool {
    if ring.len() < 3 {
        return false;
    }
    let mut inside = false;
    let mut j = ring.len() - 1;
    for i in 0..ring.len() {
        let (xi, yi) = ring[i];
        let (xj, yj) = ring[j];
        let intersects = ((yi > py) != (yj > py)) && (px < (xj - xi) * (py - yi) / (yj - yi) + xi);
        if intersects {
            inside = !inside;
        }
        j = i;
    }
    inside
}

/// Convenience: build a [`SpatialIndex`] of polygon-like items and test which
/// contain a point, returning references to the matching items.
///
/// `polygons` maps each item to a single outer ring (the index only accelerates
/// candidate selection; `point_in_ring` performs the exact test).
pub fn polygon_index_contains<T: HasBounds>(
    index: &SpatialIndex<T>,
    ring_of: impl Fn(&T) -> &[(f64, f64)],
    px: f64,
    py: f64,
) -> Vec<&T> {
    index
        .candidates_at_point(px, py)
        .into_iter()
        .filter(|item| point_in_ring(ring_of(item), px, py))
        .collect()
}

/// Validate that a bounding box is well-formed (min ≤ max on each axis).
pub fn validate_bbox(b: &Bbox) -> Result<(), GeoError> {
    if b[0] > b[2] || b[1] > b[3] {
        return Err(GeoError::spatial_index("bbox min greater than max"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Boxed(f64, f64, f64, f64);
    impl HasBounds for Boxed {
        fn bounds(&self) -> Bbox {
            [self.0, self.1, self.2, self.3]
        }
    }

    #[test]
    fn range_query() {
        let items: Vec<Boxed> = (0..100)
            .map(|i| {
                let x = (i % 10) as f64;
                let y = (i / 10) as f64;
                Boxed(x, y, x + 0.5, y + 0.5)
            })
            .collect();
        let idx = SpatialIndex::build(items);
        let q: Bbox = [1.0, 1.0, 2.0, 2.0];
        let hits = idx.range(&q);
        assert_eq!(hits.len(), 4); // (1,1),(2,1),(1,2),(2,2)
    }

    #[test]
    fn nearest_neighbor() {
        let items: Vec<Boxed> = vec![Boxed(0.0, 0.0, 1.0, 1.0), Boxed(10.0, 10.0, 11.0, 11.0)];
        let idx = SpatialIndex::build(items);
        let n = idx.nearest(2.0, 2.0).unwrap();
        assert_eq!(n.0, 0.0);
    }

    #[test]
    fn point_in_ring_basic() {
        let square = [(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)];
        assert!(point_in_ring(&square, 5.0, 5.0));
        assert!(!point_in_ring(&square, 15.0, 5.0));
    }

    #[test]
    fn empty_index() {
        let idx: SpatialIndex<Boxed> = SpatialIndex::default();
        assert!(idx.is_empty());
        assert!(idx.nearest(0.0, 0.0).is_none());
        assert!(idx.range(&[0.0, 0.0, 1.0, 1.0]).is_empty());
    }
}
