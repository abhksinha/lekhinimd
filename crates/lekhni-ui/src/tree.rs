//! Virtualized notebook sidebar tree widget.

#[cfg(feature = "alloc")]
use alloc::string::String;
#[cfg(feature = "alloc")]
use alloc::vec::Vec;

/// A node in the notebook directory hierarchy.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TreeNode {
    #[cfg(feature = "alloc")]
    pub name: String,
    pub is_dir: bool,
    pub is_expanded: bool,
    pub depth: u8,
}

/// Virtualized tree navigation state.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TreeView {
    #[cfg(feature = "alloc")]
    pub visible_nodes: Vec<TreeNode>,
    pub selected_index: usize,
}

impl TreeView {
    pub const fn new() -> Self {
        Self {
            #[cfg(feature = "alloc")]
            visible_nodes: Vec::new(),
            selected_index: 0,
        }
    }

    /// Selects previous item in tree.
    pub fn select_prev(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
        }
    }

    /// Selects next item in tree.
    #[cfg(feature = "alloc")]
    pub fn select_next(&mut self) {
        if !self.visible_nodes.is_empty() && self.selected_index + 1 < self.visible_nodes.len() {
            self.selected_index += 1;
        }
    }
}
