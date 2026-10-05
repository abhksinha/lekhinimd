//! Glyph atlas with shelf bin packing.
//!
//! Stores 8-bit grayscale coverage bitmaps for fast raster blitting.

#[cfg(feature = "alloc")]
use alloc::vec::Vec;

/// Region occupied by an allocated glyph in the atlas.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct AtlasGlyph {
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
}

/// A shelf-based 2D texture atlas storing 8-bit alpha masks.
#[cfg(feature = "alloc")]
pub struct GlyphAtlas {
    pub pixels: Vec<u8>,
    pub width: u32,
    pub height: u32,
    cursor_x: u32,
    cursor_y: u32,
    row_height: u32,
}

#[cfg(feature = "alloc")]
impl GlyphAtlas {
    pub fn new(width: u32, height: u32) -> Self {
        let size = (width * height) as usize;
        let pixels = alloc::vec![0; size];

        Self {
            pixels,
            width,
            height,
            cursor_x: 0,
            cursor_y: 0,
            row_height: 0,
        }
    }

    /// Allocates a slot on the shelf for a glyph with dimensions (w, h).
    pub fn allocate(&mut self, w: u32, h: u32) -> Option<AtlasGlyph> {
        if w > self.width || h > self.height {
            return None;
        }

        // Check if fits on current shelf
        if self.cursor_x + w > self.width {
            // Move to next shelf
            self.cursor_y += self.row_height;
            self.cursor_x = 0;
            self.row_height = 0;
        }

        if self.cursor_y + h > self.height {
            // Atlas full (in full implementation, trigger LRU row eviction)
            return None;
        }

        let glyph = AtlasGlyph {
            x: self.cursor_x as u16,
            y: self.cursor_y as u16,
            width: w as u16,
            height: h as u16,
        };

        self.cursor_x += w;
        if h > self.row_height {
            self.row_height = h;
        }

        Some(glyph)
    }

    /// Writes raw glyph alpha mask into the allocated slot.
    pub fn write_glyph(&mut self, slot: &AtlasGlyph, mask: &[u8]) {
        let gw = slot.width as usize;
        let gh = slot.height as usize;
        let sx = slot.x as usize;
        let sy = slot.y as usize;

        for row in 0..gh {
            let src_start = row * gw;
            let dst_start = (sy + row) * (self.width as usize) + sx;
            let src_row = &mask[src_start..src_start + gw];
            self.pixels[dst_start..dst_start + gw].copy_from_slice(src_row);
        }
    }

    /// Clears the entire atlas.
    pub fn clear(&mut self) {
        self.pixels.fill(0);
        self.cursor_x = 0;
        self.cursor_y = 0;
        self.row_height = 0;
    }
}
