//! Accessibility (a11y) tree generation.
//!
//! Follows Section 10 of LEKHNI_ARCHITECTURE:
//! Lazily emits accessibility nodes, roles, bounds, and text contents with zero cost when inactive.

#[cfg(feature = "alloc")]
use alloc::string::String;
#[cfg(feature = "alloc")]
use alloc::vec::Vec;
use lekhni_raster::damage::Rect;

/// Standard accessibility role tags.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum A11yRole {
    Window,
    Group,
    List,
    ListItem,
    MultilineTextInput,
    StaticText,
    Button,
}

/// An individual accessibility tree node.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct A11yNode {
    pub id: u32,
    pub role: A11yRole,
    pub bounds: Rect,
    #[cfg(feature = "alloc")]
    pub name: Option<String>,
    #[cfg(feature = "alloc")]
    pub value: Option<String>,
    #[cfg(feature = "alloc")]
    pub children: Vec<u32>,
}

/// A complete accessibility hierarchy snapshot.
#[cfg(feature = "alloc")]
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct A11yTree {
    pub nodes: Vec<A11yNode>,
    pub root_id: u32,
}

#[cfg(feature = "alloc")]
impl A11yTree {
    pub fn new(root_id: u32) -> Self {
        Self {
            nodes: Vec::new(),
            root_id,
        }
    }

    pub fn add_node(&mut self, node: A11yNode) {
        self.nodes.push(node);
    }
}
