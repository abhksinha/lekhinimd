//! External file modification tracking and conflict detection.
//!
//! Follows Section 11 of LEKHNI_ARCHITECTURE:
//! Tracks `(mtime, size, content hash of last-saved)` as file generation.
//! Clean buffer -> reload silently.
//! Dirty buffer -> conflict prompt.

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct FileGeneration {
    pub mtime: u64,
    pub size: u64,
    pub content_hash: u64,
}

impl FileGeneration {
    pub const fn new(mtime: u64, size: u64, content_hash: u64) -> Self {
        Self {
            mtime,
            size,
            content_hash,
        }
    }
}

/// Action to take when an external edit is detected.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConflictResolution {
    UpToDate,
    ReloadSilently,
    PromptConflict,
}

/// Detects external modifications by comparing file generations.
pub fn check_external_edit(
    saved_gen: &FileGeneration,
    current_mtime: u64,
    current_size: u64,
    buffer_is_dirty: bool,
) -> ConflictResolution {
    if saved_gen.mtime == current_mtime && saved_gen.size == current_size {
        ConflictResolution::UpToDate
    } else if buffer_is_dirty {
        ConflictResolution::PromptConflict
    } else {
        ConflictResolution::ReloadSilently
    }
}
