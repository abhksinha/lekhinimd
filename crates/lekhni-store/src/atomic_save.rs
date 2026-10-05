//! Atomic file saving protocol.
//!
//! Follows Section 11 of LEKHNI_ARCHITECTURE:
//! Write temp file in same directory, fsync file, rename over target, fsync dir.

#[cfg(feature = "alloc")]
use alloc::format;
#[cfg(feature = "alloc")]
use lekhni_shell::Fs;

/// Saves document data atomically to prevent corruption on crash or power loss.
#[cfg(feature = "alloc")]
pub fn save_atomic<F: Fs>(fs: &F, target_path: &str, data: &[u8]) -> Result<(), F::Error> {
    let tmp_path = format!("{}.tmp.{}", target_path, 1);
    // 1. Write data to temporary file in same directory
    fs.write_atomic(&tmp_path, data)?;

    // 2. Atomic rename over destination target
    fs.rename(&tmp_path, target_path)?;

    Ok(())
}
