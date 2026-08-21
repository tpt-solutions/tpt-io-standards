//! Hierarchical netlist flattening.
//!
//! EDIF netlists are hierarchical: a cell instantiates other cells via `(instance
//! …)`, and connectivity is expressed in terms of those instances' interface
//! ports. [`flatten`] walks the hierarchy from a top cell and produces a single
//! flat netlist of [`LeafInstance`]s wired by [`FlatNet`]s, merging nets that meet
//! at an instance boundary via a union-find over port references.
//!
//! A [`FlatRef`] with an empty `path` denotes a top-level interface port; a dotted
//! `path` denotes a port of a nested (leaf) instance, e.g. `"u1/u2"`.

use alloc::string::{String, ToString};
use alloc::vec::Vec;

use crate::error::EdifError;
use crate::model::{Edif, PortRef, View};

/// A reference to a port in the flattened netlist.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FlatRef {
    /// Instance path (`.`-separated). Empty means a top-level interface port.
    pub path: String,
    /// Port name.
    pub port: String,
}

/// A flattened net: a set of connected port references.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlatNet {
    /// Net name (inherited from whichever source net first mentioned it).
    pub name: String,
    /// Connected port references.
    pub members: Vec<FlatRef>,
}

/// A primitive (non-expandable) cell instance in the flat netlist.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LeafInstance {
    /// Hierarchical instance path (`.`-separated).
    pub path: String,
    /// Referenced library.
    pub library: String,
    /// Referenced cell.
    pub cell: String,
}

/// A fully flattened netlist.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FlatNetlist {
    /// Top cell name.
    pub top_cell: String,
    /// Top-level interface port names (in declaration order).
    pub external_ports: Vec<String>,
    /// Primitive instances.
    pub leaves: Vec<LeafInstance>,
    /// Flattened nets.
    pub nets: Vec<FlatNet>,
}

/// Flatten `edif` starting from its design's top cell.
pub fn flatten(edif: &Edif) -> core::result::Result<FlatNetlist, EdifError> {
    let design =
        edif.design.as_ref().ok_or(EdifError::Unsupported("no (design …) form to flatten from"))?;
    flatten_from(edif, &design.library_ref, &design.cell_ref)
}

/// Flatten from an explicit (library, cell) top.
pub fn flatten_from(
    edif: &Edif,
    library: &str,
    cell: &str,
) -> core::result::Result<FlatNetlist, EdifError> {
    let top = edif.find_cell(library, cell).ok_or(EdifError::Unsupported("top cell not found"))?;
    let external_ports = top
        .netlist_view()
        .map(|v| v.interface.iter().map(|p| p.name.clone()).collect())
        .unwrap_or_default();

    let expanded = expand_view(edif, library, cell, "", 0)
        .map_err(|_| EdifError::Unsupported("cycle detected while flattening hierarchy"))?;

    // Union-find over every port reference encountered.
    let mut uf = UnionFind::new();
    // For each union root, the chosen net name.
    let mut name_for_root: Vec<Option<String>> = Vec::new();

    for net in &expanded.nets {
        if net.members.is_empty() {
            continue;
        }
        let ids: Vec<usize> = net.members.iter().map(|r| uf.intern(r)).collect();
        let root = uf.find(ids[0]);
        for &id in &ids[1..] {
            let other = uf.find(id);
            uf.union(other, root);
        }
        let root = uf.find(root);
        if name_for_root.get(root).and_then(|n| n.as_ref()).is_none() {
            ensure_len(&mut name_for_root, root + 1);
            name_for_root[root] = Some(net.name.clone());
        }
    }

    // Group refs by their root.
    let mut groups: Vec<Vec<FlatRef>> = Vec::new();
    let mut group_root: Vec<usize> = Vec::new();
    for id in 0..uf.len() {
        let root = uf.find(id);
        let pos = group_root.iter().position(|&r| r == root);
        let pos = match pos {
            Some(p) => p,
            None => {
                group_root.push(root);
                groups.push(Vec::new());
                groups.len() - 1
            }
        };
        let r = uf.ref_at(id).clone();
        if !groups[pos].iter().any(|x| x == &r) {
            groups[pos].push(r);
        }
    }

    let mut out_nets = Vec::new();
    for (gi, group) in groups.iter().enumerate() {
        let root = group_root[gi];
        let name = name_for_root.get(root).cloned().flatten().unwrap_or_default();
        out_nets.push(FlatNet { name, members: group.clone() });
    }

    Ok(FlatNetlist {
        top_cell: cell.to_string(),
        external_ports,
        leaves: expanded.leaves,
        nets: out_nets,
    })
}

fn ensure_len(v: &mut Vec<Option<String>>, n: usize) {
    while v.len() < n {
        v.push(None);
    }
}

/// Result of expanding one view into leaf instances + internal nets.
struct Expanded {
    leaves: Vec<LeafInstance>,
    nets: Vec<FlatNet>,
}

fn join(path: &str, child: &str) -> String {
    if path.is_empty() {
        child.to_string()
    } else {
        alloc::format!("{path}.{child}")
    }
}

fn expand_view(
    edif: &Edif,
    library: &str,
    cell: &str,
    path: &str,
    depth: u32,
) -> core::result::Result<Expanded, ()> {
    if depth > 256 {
        return Err(());
    }
    let cell_ref = edif.find_cell(library, cell).ok_or(())?;
    let view: Option<&View> = cell_ref.netlist_view();

    let mut leaves = Vec::new();
    let mut nets: Vec<FlatNet> = Vec::new();

    if let Some(v) = view {
        for inst in &v.instances {
            let sub = expand_view(
                edif,
                &inst.library_ref,
                &inst.cell_ref,
                &join(path, &inst.name),
                depth + 1,
            )?;
            for l in sub.leaves {
                leaves.push(l);
            }
            for net in sub.nets {
                // Remap: sub's external ports (path=="") become this instance's port node;
                // everything else is prefixed with the instance name.
                let members: Vec<FlatRef> = net
                    .members
                    .into_iter()
                    .map(|r| {
                        if r.path.is_empty() {
                            FlatRef { path: inst.name.clone(), port: r.port }
                        } else {
                            FlatRef { path: join(&inst.name, &r.path), port: r.port }
                        }
                    })
                    .collect();
                nets.push(FlatNet { name: net.name, members });
            }
        }

        // This cell's own nets: external ports (no instance) get path="".
        for net in &v.nets {
            let members: Vec<FlatRef> = net
                .members
                .iter()
                .map(|pr: &PortRef| match &pr.instance {
                    None => FlatRef { path: String::new(), port: pr.port.clone() },
                    Some(name) => FlatRef { path: name.clone(), port: pr.port.clone() },
                })
                .collect();
            nets.push(FlatNet { name: net.name.clone(), members });
        }
    }

    // A cell with no instances to expand is a primitive leaf and is recorded as
    // such (its interface ports are external terminals at `path`).
    if view.map_or(true, |v| v.instances.is_empty()) {
        leaves.push(LeafInstance {
            path: path.to_string(),
            library: library.to_string(),
            cell: cell.to_string(),
        });
    }

    Ok(Expanded { leaves, nets })
}

/// Minimal union-find keyed by [`FlatRef`].
struct UnionFind {
    refs: Vec<FlatRef>,
    parent: Vec<usize>,
}

impl UnionFind {
    fn new() -> Self {
        Self { refs: Vec::new(), parent: Vec::new() }
    }

    fn len(&self) -> usize {
        self.refs.len()
    }

    fn intern(&mut self, r: &FlatRef) -> usize {
        if let Some(pos) = self.refs.iter().position(|x| x == r) {
            return pos;
        }
        let id = self.refs.len();
        self.refs.push(r.clone());
        self.parent.push(id);
        id
    }

    fn ref_at(&self, id: usize) -> &FlatRef {
        &self.refs[id]
    }

    fn find(&mut self, mut id: usize) -> usize {
        while self.parent[id] != id {
            self.parent[id] = self.parent[self.parent[id]];
            id = self.parent[id];
        }
        id
    }

    fn union(&mut self, mut a: usize, mut b: usize) {
        a = self.find(a);
        b = self.find(b);
        if a != b {
            self.parent[b] = a;
        }
    }
}
