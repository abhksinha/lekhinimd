//! Platform shell abstractions for Lekhni.
//!
//! Follows Section 13 of LEKHNI_ARCHITECTURE:
//! Core never blocks on shell. Traits isolate OS contact (windows, files, clipboard, input).

#![no_std]

#[cfg(feature = "alloc")]
extern crate alloc;

#[cfg(feature = "alloc")]
use alloc::string::String;
#[cfg(feature = "alloc")]
use alloc::vec::Vec;
use lekhni_raster::damage::Rect;

/// Filesystem interface.
pub trait Fs {
    type Error;

    /// Reads an entire file into memory (or returns mmap'd view).
    #[cfg(feature = "alloc")]
    fn read_bytes(&self, path: &str) -> Result<Vec<u8>, Self::Error>;

    /// Writes data to target file path atomically.
    fn write_atomic(&self, path: &str, data: &[u8]) -> Result<(), Self::Error>;

    /// Checks if a file exists and returns size and modification time.
    fn stat(&self, path: &str) -> Result<(u64, u64), Self::Error>;

    /// Deletes a file.
    fn remove(&self, path: &str) -> Result<(), Self::Error>;

    /// Renames/moves a file.
    fn rename(&self, from: &str, to: &str) -> Result<(), Self::Error>;
}

/// Filesystem modification change watcher.
pub trait Watcher {
    type Error;

    /// Subscribes to directory or file changes.
    fn subscribe(&mut self, path: &str) -> Result<(), Self::Error>;

    /// Polls for change events without blocking.
    fn poll_changes(&mut self) -> Option<&str>;
}

/// OS clipboard interaction.
pub trait Clipboard {
    #[cfg(feature = "alloc")]
    fn get_text(&self) -> Option<String>;

    fn set_text(&mut self, text: &str);
}

/// Monotonic clock and wake deadline scheduling.
pub trait Clock {
    /// Monotonic time in milliseconds since epoch or arbitrary start.
    fn now_ms(&self) -> u64;

    /// Requests a wake notification at `deadline_ms`.
    fn set_deadline(&mut self, deadline_ms: u64);
}

/// Display presentation surface (software or GPU).
pub trait Presenter {
    /// Returns the window/canvas dimensions (width, height) in physical pixels.
    fn size(&self) -> (u32, u32);

    /// Returns the display scale factor (e.g. 1.0, 2.0 for HiDPI).
    fn scale(&self) -> f32;

    /// Presents damaged region from the pixel buffer.
    fn present(&mut self, damage: Rect, pixels: &[u32]);
}

/// Native system dialogs.
pub trait Dialogs {
    #[cfg(feature = "alloc")]
    fn open_dir(&self, title: &str) -> Option<String>;

    fn confirm(&self, title: &str, message: &str) -> bool;
}

/// Font enumeration and byte loading.
pub trait Fonts {
    type Error;

    /// Loads font data bytes for the given font identifier.
    #[cfg(feature = "alloc")]
    fn load_font_bytes(&self, name: &str) -> Result<Vec<u8>, Self::Error>;

    /// Returns fallback font identifier for a given Unicode codepoint.
    fn fallback_for(&self, codepoint: char) -> Option<&'static str>;
}
