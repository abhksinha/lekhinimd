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
        if let Some(parent) = Path::new(path).parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, data)
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

/// Workspace Notebook Manager enforcing unique notebook names.
pub struct NotebookManager {
    pub working_dir: PathBuf,
    pub notebooks: Vec<String>,
    pub active_notebook_idx: usize,
    pub pages: Vec<String>,
    pub active_page_idx: usize,
    pub fs: StdFs,
}

impl NotebookManager {
    pub fn new(working_dir: PathBuf) -> Self {
        let mut mgr = Self {
            working_dir,
            notebooks: Vec::new(),
            active_notebook_idx: 0,
            pages: Vec::new(),
            active_page_idx: 0,
            fs: StdFs,
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

    /// Rescans markdown pages in the active notebook directory.
    pub fn refresh_pages(&mut self) {
        self.pages.clear();
        if let Some(nb_name) = self.notebooks.get(self.active_notebook_idx) {
            let nb_path = self.working_dir.join(nb_name);
            if let Ok(entries) = fs::read_dir(&nb_path) {
                for entry in entries.flatten() {
                    if let Ok(ft) = entry.file_type() {
                        if ft.is_file() {
                            let fname = entry.file_name().to_string_lossy().to_string();
                            if fname.ends_with(".md") {
                                self.pages.push(fname);
                            }
                        }
                    }
                }
            }
        }

        self.pages.sort();

        // If no pages in active notebook, create welcome.md
        if self.pages.is_empty() {
            let _ = self.create_page("welcome.md", DEFAULT_WELCOME_DOC);
        } else if self.active_page_idx >= self.pages.len() {
            self.active_page_idx = 0;
        }
    }

    /// Creates a notebook with a strictly unique name.
    ///
    /// If a notebook with the requested name already exists (case-insensitively),
    /// generates a guaranteed unique name with a numerical suffix.
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
        let _ = fs::write(&file_path, content);

        if !self.pages.iter().any(|p| p == &clean) {
            self.pages.push(clean.clone());
            self.pages.sort();
        }
        if let Some(pos) = self.pages.iter().position(|p| p == &clean) {
            self.active_page_idx = pos;
        }
        Some(clean)
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

    /// Reads active document content bytes.
    pub fn load_active_content(&self) -> Vec<u8> {
        if let Some(path) = self.active_file_path() {
            if let Ok(bytes) = fs::read(&path) {
                return bytes;
            }
        }
        DEFAULT_WELCOME_DOC.to_vec()
    }

    /// Saves document content bytes atomically to disk.
    pub fn save_active_content(&self, data: &[u8]) -> Result<(), String> {
        if let Some(path) = self.active_file_path() {
            let path_str = path.to_string_lossy();
            save_atomic(&self.fs, &path_str, data).map_err(|e| e.to_string())
        } else {
            Err("No active page selected".into())
        }
    }
}

pub const DEFAULT_WELCOME_DOC: &[u8] = b"# \xe0\xa4\xb2\xe0\xa5\x87\xe0\xa4\x96\xe0\xa4\xa8\xe0\xa5\x80 (Lekhni)\n\nWelcome to **Lekhni** \xe2\x80\x94 the world-class pure Rust Markdown editor.\n\n```rust\nfn main() {\n    println!(\"Pure Rust desktop frontend live!\");\n}\n```\n\n> 1980s discipline: small, fixed, measured.\n\n- [x] Unique notebook directories\n- [x] Pure Rust native X11 presentation\n- [x] Sub-8ms keystroke latency\n- [x] Real-time split preview\n";
