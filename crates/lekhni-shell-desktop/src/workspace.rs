//! Working directory, multi-notebook, and page storage manager.
//!
//! Enforces:
//! 1. User selection of working directory on first launch.
//! 2. Storing data notebook-wise within the working directory.
//! 3. Strict uniqueness of notebook names (no duplicate notebook names).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use lekhni_shell::Fs;
use lekhni_store::atomic_save::save_atomic;

/// Standard filesystem driver for Lekhni desktop shell.
pub struct StdFs;

impl Fs for StdFs {
    type Error = std::io::Error;

    fn read_bytes(&self, path: &str) -> Result<Vec<u8>, Self::Error> {
        fs::read(path)
    }

    fn write_atomic(&self, path: &str, data: &[u8]) -> Result<(), Self::Error> {
        use std::io::Write;
        if let Some(parent) = Path::new(path).parent() {
            fs::create_dir_all(parent)?;
        }
        let mut file = fs::File::create(path)?;
        file.write_all(data)?;
        file.sync_all()?;
        Ok(())
    }

    fn stat(&self, path: &str) -> Result<(u64, u64), Self::Error> {
        let meta = fs::metadata(path)?;
        let modified = meta.modified()?
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        Ok((meta.len(), modified))
    }

    fn remove(&self, path: &str) -> Result<(), Self::Error> {
        fs::remove_file(path)
    }

    fn rename(&self, from: &str, to: &str) -> Result<(), Self::Error> {
        fs::rename(from, to)
    }
}

/// Retrieves or prompts the user for the Lekhni working directory on first launch.
pub fn get_or_init_working_dir() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    let config_dir = PathBuf::from(&home).join(".config").join("lekhni");
    let config_file = config_dir.join("config.json");

    if let Ok(content) = fs::read_to_string(&config_file) {
        if let Some(path) = parse_json_string_field(&content, "working_dir") {
            let p = PathBuf::from(path);
            if p.is_dir() || fs::create_dir_all(&p).is_ok() {
                return p;
            }
        }
    }

    // First time launch: Prompt user to select directory via zenity or fallback
    let mut selected_dir = None;
    if Path::new("/usr/bin/zenity").exists() {
        if let Ok(output) = Command::new("zenity")
            .args([
                "--file-selection",
                "--directory",
                "--title=Select Lekhni Working Directory",
                "--timeout=3",
            ])
            .output()
        {
            if output.status.success() {
                let s = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !s.is_empty() {
                    selected_dir = Some(PathBuf::from(s));
                }
            }
        }
    }

    let working_dir = selected_dir.unwrap_or_else(|| {
        PathBuf::from(&home).join("LekhniNotes")
    });

    let _ = fs::create_dir_all(&working_dir);
    let _ = fs::create_dir_all(&config_dir);

    let config_content = format!(
        "{{\n  \"working_dir\": \"{}\"\n}}\n",
        working_dir.to_string_lossy().replace('\\', "\\\\")
    );
    let _ = fs::write(&config_file, config_content);

    working_dir
}

/// Helper to parse simple JSON string fields without external crates.
fn parse_json_string_field(json: &str, key: &str) -> Option<String> {
    let pattern = format!("\"{}\"", key);
    let key_pos = json.find(&pattern)?;
    let colon_pos = json[key_pos..].find(':')? + key_pos;
    let quote_start = json[colon_pos..].find('"')? + colon_pos + 1;
    let quote_end = json[quote_start..].find('"')? + quote_start;
    Some(json[quote_start..quote_end].to_string())
}

use lekhni_index::trigram::TrigramIndex;
use lekhni_store::external_edit::{check_external_edit, ConflictResolution, FileGeneration};
use lekhni_store::notebook::PageSortOrder;

/// Search match hit result from Trigram-backed index.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SearchHit {
    pub page_idx: usize,
    pub page_name: String,
    pub line_num: usize,
    pub line_text: String,
    pub byte_offset: usize,
}

/// Outline item representing a document heading.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OutlineItem {
    pub level: u8,
    pub title: String,
    pub byte_offset: usize,
}

/// Computes fast FNV-1a 64-bit content hash.
pub fn fnv1a_64(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf29ce484222325u64;
    for &b in bytes {
        hash ^= b as u64;
        hash = hash.wrapping_mul(0x100000001b3u64);
    }
    hash
}

/// Workspace Notebook Manager enforcing unique notebook names.
pub struct NotebookManager {
    pub working_dir: PathBuf,
    pub notebooks: Vec<String>,
    pub active_notebook_idx: usize,
    // Structure of Arrays (SoA) layout for high L1/L2 cache locality:
    pub pages: Vec<String>,
    pub page_titles: Vec<String>,
    pub page_mtimes: Vec<u64>,
    pub page_words: Vec<usize>,
    pub active_page_idx: usize,
    pub fs: StdFs,
    pub sort_order: PageSortOrder,
    pub last_saved_gen: Option<FileGeneration>,
}

impl NotebookManager {
    pub fn new(working_dir: PathBuf) -> Self {
        let mut mgr = Self {
            working_dir,
            notebooks: Vec::new(),
            active_notebook_idx: 0,
            pages: Vec::new(),
            page_titles: Vec::new(),
            page_mtimes: Vec::new(),
            page_words: Vec::new(),
            active_page_idx: 0,
            fs: StdFs,
            sort_order: PageSortOrder::NameAsc,
            last_saved_gen: None,
        };
        mgr.refresh_notebooks();
        mgr
    }

    /// Rescans notebooks inside working directory.
    pub fn refresh_notebooks(&mut self) {
        self.notebooks.clear();
        if let Ok(entries) = fs::read_dir(&self.working_dir) {
            for entry in entries.flatten() {
                if let Ok(ft) = entry.file_type() {
                    if ft.is_dir() {
                        let name = entry.file_name().to_string_lossy().to_string();
                        if !name.starts_with('.') {
                            self.notebooks.push(name);
                        }
                    }
                }
            }
        }

        self.notebooks.sort();

        // If no notebook exists, initialize default unique "General" notebook
        if self.notebooks.is_empty() {
            let _ = self.create_notebook("General");
        } else {
            if self.active_notebook_idx >= self.notebooks.len() {
                self.active_notebook_idx = 0;
            }
            self.refresh_pages();
        }
    }

    /// Rescans markdown pages in the active notebook directory (supports nested folders).
    pub fn refresh_pages(&mut self) {
        let active_path = self.pages.get(self.active_page_idx).cloned();
        self.pages.clear();
        self.page_titles.clear();
        self.page_mtimes.clear();
        self.page_words.clear();

        if let Some(nb_name) = self.notebooks.get(self.active_notebook_idx) {
            let nb_path = self.working_dir.join(nb_name);
            let mut scanned = Vec::new();
            scan_markdown_files(&nb_path, "", &mut scanned);

            for rel_path in scanned {
                let full_path = nb_path.join(&rel_path);
                let (mtime, words, title) = if let Ok(bytes) = fs::read(&full_path) {
                    let mtime = fs::metadata(&full_path)
                        .and_then(|m| m.modified())
                        .map(|t| t.duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs())
                        .unwrap_or(0);
                    let s = String::from_utf8_lossy(&bytes);
                    let words = s.split_whitespace().count();
                    let title = s.lines()
                        .find(|l| l.trim_start().starts_with("# "))
                        .map(|l| l.trim_start().trim_start_matches("# ").trim().to_string())
                        .unwrap_or_else(|| {
                            Path::new(&rel_path)
                                .file_stem()
                                .and_then(|n| n.to_str())
                                .unwrap_or("Note")
                                .to_string()
                        });
                    (mtime, words, title)
                } else {
                    (0, 0, rel_path.clone())
                };

                self.pages.push(rel_path);
                self.page_titles.push(title);
                self.page_mtimes.push(mtime);
                self.page_words.push(words);
            }
        }

        self.apply_sort();

        // If no pages in active notebook, create welcome.md
        if self.pages.is_empty() {
            let _ = self.create_page("welcome.md", DEFAULT_WELCOME_DOC);
        } else if let Some(ref prev) = active_path {
            if let Some(pos) = self.pages.iter().position(|p| p == prev) {
                self.active_page_idx = pos;
            } else if self.active_page_idx >= self.pages.len() {
                self.active_page_idx = 0;
            }
        } else if self.active_page_idx >= self.pages.len() {
            self.active_page_idx = 0;
        }
    }

    /// Sorts all SoA page arrays in tandem according to `sort_order`.
    pub fn apply_sort(&mut self) {
        if self.pages.len() <= 1 {
            return;
        }

        let mut indices: Vec<usize> = (0..self.pages.len()).collect();
        match self.sort_order {
            PageSortOrder::NameAsc => indices.sort_by(|&a, &b| self.pages[a].cmp(&self.pages[b])),
            PageSortOrder::NameDesc => indices.sort_by(|&a, &b| self.pages[b].cmp(&self.pages[a])),
            PageSortOrder::DateModifiedDesc => indices.sort_by(|&a, &b| self.page_mtimes[b].cmp(&self.page_mtimes[a])),
            PageSortOrder::DateModifiedAsc => indices.sort_by(|&a, &b| self.page_mtimes[a].cmp(&self.page_mtimes[b])),
        }

        let orig_pages = self.pages.clone();
        let orig_titles = self.page_titles.clone();
        let orig_mtimes = self.page_mtimes.clone();
        let orig_words = self.page_words.clone();

        for (new_pos, &old_idx) in indices.iter().enumerate() {
            self.pages[new_pos] = orig_pages[old_idx].clone();
            self.page_titles[new_pos] = orig_titles[old_idx].clone();
            self.page_mtimes[new_pos] = orig_mtimes[old_idx];
            self.page_words[new_pos] = orig_words[old_idx];
        }
    }

    /// Cycles to the next sort order.
    pub fn toggle_sort(&mut self) -> &'static str {
        self.sort_order = match self.sort_order {
            PageSortOrder::NameAsc => PageSortOrder::NameDesc,
            PageSortOrder::NameDesc => PageSortOrder::DateModifiedDesc,
            PageSortOrder::DateModifiedDesc => PageSortOrder::DateModifiedAsc,
            PageSortOrder::DateModifiedAsc => PageSortOrder::NameAsc,
        };
        self.apply_sort();
        match self.sort_order {
            PageSortOrder::NameAsc => "Name (A-Z)",
            PageSortOrder::NameDesc => "Name (Z-A)",
            PageSortOrder::DateModifiedDesc => "Date (Newest)",
            PageSortOrder::DateModifiedAsc => "Date (Oldest)",
        }
    }

    /// Creates a notebook with a strictly unique name.
    pub fn create_notebook(&mut self, requested_name: &str) -> String {
        let clean_name = requested_name.trim();
        let base_name = if clean_name.is_empty() { "Notebook" } else { clean_name };

        let mut candidate = base_name.to_string();
        let mut counter = 1;

        while self.notebook_exists(&candidate) {
            candidate = format!("{}_{}", base_name, counter);
            counter += 1;
        }

        let nb_path = self.working_dir.join(&candidate);
        let meta_path = nb_path.join(".lekhni");
        let _ = fs::create_dir_all(&meta_path);

        let initial_doc = format!(
            "# {}\n\nWelcome to your new notebook `{}`.\n\n- [ ] Add first note\n",
            candidate, candidate
        );
        let page_path = nb_path.join("welcome.md");
        let _ = fs::write(&page_path, initial_doc.as_bytes());

        self.notebooks.push(candidate.clone());
        self.notebooks.sort();
        if let Some(pos) = self.notebooks.iter().position(|n| n == &candidate) {
            self.active_notebook_idx = pos;
        }
        self.refresh_pages();
        candidate
    }

    /// Checks if a notebook name exists (case-insensitively).
    pub fn notebook_exists(&self, name: &str) -> bool {
        self.notebooks
            .iter()
            .any(|n| n.eq_ignore_ascii_case(name))
    }

    /// Creates a new page in active notebook.
    pub fn create_page(&mut self, page_name: &str, content: &[u8]) -> Option<String> {
        let nb_name = self.notebooks.get(self.active_notebook_idx)?;
        let mut clean = page_name.trim().to_string();
        if !clean.ends_with(".md") {
            clean.push_str(".md");
        }

        let file_path = self.working_dir.join(nb_name).join(&clean);
        if let Some(parent) = file_path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let _ = fs::write(&file_path, content);

        self.refresh_pages();
        if let Some(pos) = self.pages.iter().position(|p| p == &clean) {
            self.active_page_idx = pos;
        }
        Some(clean)
    }

    /// Deletes a page from the active notebook.
    pub fn delete_page(&mut self, page_name: &str) -> Result<(), String> {
        let nb_name = self.notebooks.get(self.active_notebook_idx).ok_or("No active notebook")?;
        let file_path = self.working_dir.join(nb_name).join(page_name);
        if file_path.exists() {
            fs::remove_file(&file_path).map_err(|e| e.to_string())?;
        }
        self.refresh_pages();
        Ok(())
    }

    /// Renames a page in the active notebook.
    pub fn rename_page(&mut self, old_name: &str, new_name: &str) -> Result<String, String> {
        let nb_name = self.notebooks.get(self.active_notebook_idx).ok_or("No active notebook")?;
        let mut clean_new = new_name.trim().to_string();
        if !clean_new.ends_with(".md") {
            clean_new.push_str(".md");
        }

        let old_path = self.working_dir.join(nb_name).join(old_name);
        let new_path = self.working_dir.join(nb_name).join(&clean_new);
        if let Some(parent) = new_path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        fs::rename(&old_path, &new_path).map_err(|e| e.to_string())?;
        self.refresh_pages();
        if let Some(pos) = self.pages.iter().position(|p| p == &clean_new) {
            self.active_page_idx = pos;
        }
        Ok(clean_new)
    }

    /// Moves a page to another notebook.
    pub fn move_page(&mut self, page_name: &str, target_notebook: &str) -> Result<String, String> {
        let curr_nb = self.notebooks.get(self.active_notebook_idx).ok_or("No active notebook")?;
        let old_path = self.working_dir.join(curr_nb).join(page_name);
        let target_dir = self.working_dir.join(target_notebook);
        fs::create_dir_all(&target_dir).map_err(|e| e.to_string())?;
        let new_path = target_dir.join(page_name);
        if let Some(parent) = new_path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        fs::rename(&old_path, &new_path).map_err(|e| e.to_string())?;
        self.refresh_pages();
        Ok(format!("{}/{}", target_notebook, page_name))
    }

    /// Saves binary image bytes into the notebook's assets/ folder and returns relative Markdown path.
    pub fn save_asset(&self, filename: &str, bytes: &[u8]) -> Result<String, String> {
        let nb_dir = self.active_notebook_dir().ok_or("No active notebook")?;
        let assets_dir = nb_dir.join("assets");
        fs::create_dir_all(&assets_dir).map_err(|e| e.to_string())?;
        let clean_fname = Path::new(filename)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("image.png");
        let asset_path = assets_dir.join(clean_fname);
        fs::write(&asset_path, bytes).map_err(|e| e.to_string())?;
        Ok(format!("assets/{}", clean_fname))
    }

    /// Searches active notebook notes using the Trigram postings index.
    pub fn search_notes(&self, query: &str) -> Vec<SearchHit> {
        let clean_query = query.trim();
        if clean_query.is_empty() {
            return Vec::new();
        }
        let query_lower = clean_query.to_lowercase();
        let query_bytes = query_lower.as_bytes();

        let mut trigram_idx = TrigramIndex::new();
        let mut file_contents = Vec::new();

        let nb_dir = match self.active_notebook_dir() {
            Some(d) => d,
            None => return Vec::new(),
        };

        for (idx, page) in self.pages.iter().enumerate() {
            let path = nb_dir.join(page);
            let content = fs::read_to_string(&path).unwrap_or_default();
            trigram_idx.index_doc(idx as u32, content.as_bytes());
            file_contents.push((page.clone(), content));
        }

        let candidate_ids: Vec<u32> = if query_bytes.len() >= 3 {
            trigram_idx.query_candidates(query_bytes)
        } else {
            (0..self.pages.len() as u32).collect()
        };

        let mut hits = Vec::new();
        for doc_id in candidate_ids {
            let idx = doc_id as usize;
            if let Some((page_name, content)) = file_contents.get(idx) {
                let mut byte_offset = 0;
                for (line_idx, line) in content.lines().enumerate() {
                    let line_lower = line.to_lowercase();
                    if let Some(col) = line_lower.find(&query_lower) {
                        hits.push(SearchHit {
                            page_idx: idx,
                            page_name: page_name.clone(),
                            line_num: line_idx + 1,
                            line_text: line.trim().to_string(),
                            byte_offset: byte_offset + col,
                        });
                    }
                    byte_offset += line.len() + 1;
                }
            }
        }
        hits
    }

    /// Returns all backlinks pointing to the specified target note.
    pub fn get_backlinks(&self, target_page: &str) -> Vec<String> {
        let nb_dir = match self.active_notebook_dir() {
            Some(d) => d,
            None => return Vec::new(),
        };
        let target_clean = target_page.trim_end_matches(".md");
        let mut referring_notes = Vec::new();

        for page in &self.pages {
            if page == target_page {
                continue;
            }
            let path = nb_dir.join(page);
            if let Ok(content) = fs::read_to_string(&path) {
                let link_pattern = format!("]({})", target_page);
                let link_pattern_no_ext = format!("]({}.md)", target_clean);
                let wiki_pattern = format!("[[{}]]", target_clean);
                let wiki_pipe = format!("[[{}|", target_clean);
                if content.contains(&link_pattern)
                    || content.contains(&link_pattern_no_ext)
                    || content.contains(&wiki_pattern)
                    || content.contains(&wiki_pipe)
                {
                    referring_notes.push(page.clone());
                }
            }
        }
        referring_notes
    }

    /// Checks if the active page was modified externally on disk.
    pub fn check_active_external_edit(&self, is_dirty: bool) -> ConflictResolution {
        if let (Some(gen), Some(path)) = (self.last_saved_gen, self.active_file_path()) {
            if let Ok(meta) = fs::metadata(&path) {
                let mtime = meta.modified()
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).map_err(|_| std::io::ErrorKind::Other.into()))
                    .map(|d| d.as_secs())
                    .unwrap_or(0);
                let size = meta.len();
                check_external_edit(&gen, mtime, size, is_dirty)
            } else {
                ConflictResolution::UpToDate
            }
        } else {
            ConflictResolution::UpToDate
        }
    }

    /// Gets the current active notebook directory path.
    pub fn active_notebook_dir(&self) -> Option<PathBuf> {
        let nb_name = self.notebooks.get(self.active_notebook_idx)?;
        Some(self.working_dir.join(nb_name))
    }

    /// Gets the current active page file path.
    pub fn active_file_path(&self) -> Option<PathBuf> {
        let nb_name = self.notebooks.get(self.active_notebook_idx)?;
        let page_name = self.pages.get(self.active_page_idx)?;
        Some(self.working_dir.join(nb_name).join(page_name))
    }

    /// Reads active document content bytes and records its FileGeneration.
    pub fn load_active_content(&mut self) -> Vec<u8> {
        if let Some(path) = self.active_file_path() {
            if let Ok(bytes) = fs::read(&path) {
                let mtime = fs::metadata(&path)
                    .and_then(|m| m.modified())
                    .map(|t| t.duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs())
                    .unwrap_or(0);
                self.last_saved_gen = Some(FileGeneration::new(mtime, bytes.len() as u64, fnv1a_64(&bytes)));
                return bytes;
            }
        }
        DEFAULT_WELCOME_DOC.to_vec()
    }

    /// Saves document content bytes atomically to disk and updates FileGeneration.
    pub fn save_active_content(&mut self, data: &[u8]) -> Result<(), String> {
        if let Some(path) = self.active_file_path() {
            let path_str = path.to_string_lossy();
            save_atomic(&self.fs, &path_str, data).map_err(|e| e.to_string())?;
            let mtime = fs::metadata(&path)
                .and_then(|m| m.modified())
                .map(|t| t.duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs())
                .unwrap_or(0);
            self.last_saved_gen = Some(FileGeneration::new(mtime, data.len() as u64, fnv1a_64(data)));
            Ok(())
        } else {
            Err("No active page selected".into())
        }
    }
}

/// Recursively scans markdown files inside a directory, ignoring hidden directories.
fn scan_markdown_files(root: &Path, rel_prefix: &str, out: &mut Vec<String>) {
    if let Ok(entries) = fs::read_dir(root) {
        for entry in entries.flatten() {
            if let Ok(ft) = entry.file_type() {
                let fname = entry.file_name().to_string_lossy().to_string();
                if fname.starts_with('.') || fname == "assets" {
                    continue;
                }
                let rel = if rel_prefix.is_empty() {
                    fname.clone()
                } else {
                    format!("{}/{}", rel_prefix, fname)
                };
                if ft.is_dir() {
                    scan_markdown_files(&entry.path(), &rel, out);
                } else if ft.is_file() && fname.ends_with(".md") {
                    out.push(rel);
                }
            }
        }
    }
}

pub const DEFAULT_WELCOME_DOC: &[u8] = b"# \xe0\xa4\xb2\xe0\xa5\x87\xe0\xa4\x96\xe0\xa4\xa8\xe0\xa5\x80 (Lekhni)\n\nWelcome to **Lekhni** \xe2\x80\x94 the world-class pure Rust Markdown editor.\n\n```rust\nfn main() {\n    println!(\"Pure Rust desktop frontend live!\");\n}\n```\n\n> 1980s discipline: small, fixed, measured.\n\n- [x] Unique notebook directories\n- [x] Pure Rust native X11 presentation\n- [x] Sub-8ms keystroke latency\n- [x] Real-time split preview\n";
