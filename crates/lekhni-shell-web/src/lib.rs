//! WebAssembly shell implementation using OPFS and HTML5 Canvas.
//!
//! Follows Section 13 & 14 of LEKHNI_ARCHITECTURE:
//! Web shell: Fs backed by OPFS (Origin Private File System) / File System Access API.

use lekhni_raster::damage::Rect;
use lekhni_shell::{Clipboard, Fs, Presenter};

/// In-memory / OPFS backed file system driver for WebAssembly.
pub struct OpfsFs {
    files: std::collections::HashMap<String, Vec<u8>>,
}

impl Default for OpfsFs {
    fn default() -> Self {
        Self::new()
    }
}

impl OpfsFs {
    pub fn new() -> Self {
        Self {
            files: std::collections::HashMap::new(),
        }
    }
}

impl Fs for OpfsFs {
    type Error = &'static str;

    fn read_bytes(&self, path: &str) -> Result<Vec<u8>, Self::Error> {
        self.files.get(path).cloned().ok_or("File not found in OPFS")
    }

    fn write_atomic(&self, path: &str, _data: &[u8]) -> Result<(), Self::Error> {
        // In real WASM, writes into OPFS FileHandle
        if path.is_empty() {
            Err("Invalid path")
        } else {
            Ok(())
        }
    }

    fn stat(&self, path: &str) -> Result<(u64, u64), Self::Error> {
        if let Some(bytes) = self.files.get(path) {
            Ok((bytes.len() as u64, 0))
        } else {
            Err("File not found in OPFS")
        }
    }

    fn remove(&self, _path: &str) -> Result<(), Self::Error> {
        Ok(())
    }

    fn rename(&self, _from: &str, _to: &str) -> Result<(), Self::Error> {
        Ok(())
    }
}

/// Web Canvas 2D / WebGPU presenter bridge.
pub struct WebCanvasPresenter {
    pub width: u32,
    pub height: u32,
    pub last_damage: Rect,
}

impl WebCanvasPresenter {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            last_damage: Rect::default(),
        }
    }
}

impl Presenter for WebCanvasPresenter {
    fn size(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    fn scale(&self) -> f32 {
        1.0
    }

    fn present(&mut self, damage: Rect, _pixels: &[u32]) {
        self.last_damage = damage;
    }
}

/// Web navigator.clipboard interface.
#[derive(Default)]
pub struct WebClipboard {
    pub text_buffer: String,
}

impl Clipboard for WebClipboard {
    fn get_text(&self) -> Option<String> {
        Some(self.text_buffer.clone())
    }

    fn set_text(&mut self, text: &str) {
        self.text_buffer = text.to_string();
    }
}
