//! Directory-based notebook document model.

#[cfg(feature = "alloc")]
use alloc::string::String;
#[cfg(feature = "alloc")]
use alloc::vec::Vec;

/// Sorting order for notebook pages.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum PageSortOrder {
    #[default]
    NameAsc,
    NameDesc,
    DateModifiedDesc,
    DateModifiedAsc,
}

/// Metadata record for a Markdown page inside a notebook.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NotebookPage {
    #[cfg(feature = "alloc")]
    pub relative_path: String,
    #[cfg(feature = "alloc")]
    pub title: String,
    pub is_dirty: bool,
    pub mtime: u64,
    pub word_count: usize,
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

    /// Adds a page to the notebook with basic title.
    pub fn add_page(&mut self, relative_path: String, title: String) {
        self.add_page_with_meta(relative_path, title, 0, 0);
    }

    /// Adds a page with full metadata.
    pub fn add_page_with_meta(&mut self, relative_path: String, title: String, mtime: u64, word_count: usize) {
        self.pages.push(NotebookPage {
            relative_path,
            title,
            is_dirty: false,
            mtime,
            word_count,
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

    /// Deletes a page from the model by relative path.
    pub fn delete_page(&mut self, relative_path: &str) -> bool {
        if let Some(pos) = self.pages.iter().position(|p| p.relative_path == relative_path) {
            self.pages.remove(pos);
            if self.pages.is_empty() {
                self.active_page_index = None;
            } else if let Some(idx) = self.active_page_index {
                if idx >= self.pages.len() {
                    self.active_page_index = Some(self.pages.len() - 1);
                }
            }
            true
        } else {
            false
        }
    }

    /// Renames a page in the model.
    pub fn rename_page(&mut self, old_path: &str, new_path: &str, new_title: Option<&str>) -> bool {
        if let Some(page) = self.pages.iter_mut().find(|p| p.relative_path == old_path) {
            page.relative_path = new_path.into();
            if let Some(t) = new_title {
                page.title = t.into();
            }
            true
        } else {
            false
        }
    }

    /// Moves a page to a new relative path.
    pub fn move_page(&mut self, old_path: &str, new_path: &str) -> bool {
        self.rename_page(old_path, new_path, None)
    }

    /// Sorts pages according to the specified sort order.
    pub fn sort_pages(&mut self, order: PageSortOrder) {
        let active_path = self.active_page().map(|p| p.relative_path.clone());
        match order {
            PageSortOrder::NameAsc => self.pages.sort_by(|a, b| a.relative_path.cmp(&b.relative_path)),
            PageSortOrder::NameDesc => self.pages.sort_by(|a, b| b.relative_path.cmp(&a.relative_path)),
            PageSortOrder::DateModifiedDesc => self.pages.sort_by(|a, b| b.mtime.cmp(&a.mtime)),
            PageSortOrder::DateModifiedAsc => self.pages.sort_by(|a, b| a.mtime.cmp(&b.mtime)),
        }
        if let Some(path) = active_path {
            self.select_page(&path);
        }
    }
}
