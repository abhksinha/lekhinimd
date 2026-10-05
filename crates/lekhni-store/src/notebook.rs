//! Directory-based notebook document model.

#[cfg(feature = "alloc")]
use alloc::string::String;
#[cfg(feature = "alloc")]
use alloc::vec::Vec;

/// Metadata record for a Markdown page inside a notebook.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NotebookPage {
    #[cfg(feature = "alloc")]
    pub relative_path: String,
    #[cfg(feature = "alloc")]
    pub title: String,
    pub is_dirty: bool,
}

/// Notebook structure representing a directory of Markdown notes.
#[cfg(feature = "alloc")]
pub struct NotebookModel {
    pub root_path: String,
    pub pages: Vec<NotebookPage>,
    pub active_page_index: Option<usize>,
}

#[cfg(feature = "alloc")]
impl NotebookModel {
    pub fn new(root_path: String) -> Self {
        Self {
            root_path,
            pages: Vec::new(),
            active_page_index: None,
        }
    }

    /// Adds a page to the notebook.
    pub fn add_page(&mut self, relative_path: String, title: String) {
        self.pages.push(NotebookPage {
            relative_path,
            title,
            is_dirty: false,
        });
        if self.active_page_index.is_none() {
            self.active_page_index = Some(0);
        }
    }

    /// Sets active page by relative path.
    pub fn select_page(&mut self, relative_path: &str) -> bool {
        if let Some(pos) = self.pages.iter().position(|p| p.relative_path == relative_path) {
            self.active_page_index = Some(pos);
            true
        } else {
            false
        }
    }

    /// Returns the currently active page.
    pub fn active_page(&self) -> Option<&NotebookPage> {
        self.active_page_index.and_then(|idx| self.pages.get(idx))
    }
}
