//! Crash recovery journal manager.
//!
//! Follows Section 11 of LEKHNI_ARCHITECTURE:
//! Periodic journal of dirty buffers in `.lekhni/recover/` offered on next open.

#[cfg(feature = "alloc")]
use alloc::format;
#[cfg(feature = "alloc")]
use alloc::string::String;
#[cfg(feature = "alloc")]
use lekhni_base::hash::fnv1a_64;
#[cfg(feature = "alloc")]
use lekhni_shell::Fs;

pub struct RecoveryJournal;

impl RecoveryJournal {
    /// Constructs recovery journal path for a notebook page.
    #[cfg(feature = "alloc")]
    pub fn journal_path(notebook_root: &str, relative_path: &str) -> String {
        let hash = fnv1a_64(relative_path.as_bytes());
        format!("{}/.lekhni/recover/{:x}.rec", notebook_root, hash)
    }

    /// Writes a recovery snapshot for a dirty page.
    #[cfg(feature = "alloc")]
    pub fn write_snapshot<F: Fs>(
        fs: &F,
        notebook_root: &str,
        relative_path: &str,
        dirty_bytes: &[u8],
    ) -> Result<(), F::Error> {
        let path = Self::journal_path(notebook_root, relative_path);
        fs.write_atomic(&path, dirty_bytes)
    }

    /// Discards a recovery snapshot after clean save.
    #[cfg(feature = "alloc")]
    pub fn discard_snapshot<F: Fs>(
        fs: &F,
        notebook_root: &str,
        relative_path: &str,
    ) -> Result<(), F::Error> {
        let path = Self::journal_path(notebook_root, relative_path);
        let _ = fs.remove(&path);
        Ok(())
    }
}
