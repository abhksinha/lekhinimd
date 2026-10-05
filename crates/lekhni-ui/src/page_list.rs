//! Virtualized page list widget for displaying Markdown documents in a folder.

#[cfg(feature = "alloc")]
use alloc::string::String;
#[cfg(feature = "alloc")]
use alloc::vec::Vec;

/// Item representing a note in the page list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PageListItem {
    #[cfg(feature = "alloc")]
    pub title: String,
    #[cfg(feature = "alloc")]
    pub relative_path: String,
    pub is_dirty: bool,
}

/// Virtualized page list navigation state.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PageListView {
    #[cfg(feature = "alloc")]
    pub items: Vec<PageListItem>,
    pub selected_index: usize,
}

impl PageListView {
    pub const fn new() -> Self {
        Self {
            #[cfg(feature = "alloc")]
            items: Vec::new(),
            selected_index: 0,
        }
    }

    pub fn select_prev(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
        }
    }

    #[cfg(feature = "alloc")]
    pub fn select_next(&mut self) {
        if !self.items.is_empty() && self.selected_index + 1 < self.items.len() {
            self.selected_index += 1;
        }
    }
}
