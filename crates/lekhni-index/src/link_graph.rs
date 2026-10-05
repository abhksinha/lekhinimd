//! Bidirectional document link graph.
//!
//! Follows Section 4.3 of LEKHNI_ARCHITECTURE:
//! Edge list (src u32, dst u32) sorted both ways for fast forward and backlink resolution.

#[cfg(feature = "alloc")]
use alloc::vec::Vec;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct LinkEdge {
    pub src: u32,
    pub dst: u32,
}

/// Link graph tracking forward links and backlinks between documents.
#[cfg(feature = "alloc")]
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LinkGraph {
    forward_edges: Vec<LinkEdge>,
    backlink_edges: Vec<LinkEdge>,
}

#[cfg(feature = "alloc")]
impl LinkGraph {
    pub fn new() -> Self {
        Self {
            forward_edges: Vec::new(),
            backlink_edges: Vec::new(),
        }
    }

    /// Adds a link from src document to dst document.
    pub fn add_link(&mut self, src: u32, dst: u32) {
        self.forward_edges.push(LinkEdge { src, dst });
        self.backlink_edges.push(LinkEdge { src: dst, dst: src }); // Inverted
    }

    /// Sorts edges to enable binary search for forward links and backlinks.
    pub fn finalize(&mut self) {
        self.forward_edges.sort_unstable();
        self.backlink_edges.sort_unstable();
    }

    /// Returns all forward link target document IDs from `src`.
    pub fn forward_links(&self, src: u32) -> Vec<u32> {
        self.forward_edges
            .iter()
            .filter(|e| e.src == src)
            .map(|e| e.dst)
            .collect()
    }

    /// Returns all backlink source document IDs pointing to `dst`.
    pub fn backlinks(&self, dst: u32) -> Vec<u32> {
        self.backlink_edges
            .iter()
            .filter(|e| e.src == dst)
            .map(|e| e.dst)
            .collect()
    }
}
