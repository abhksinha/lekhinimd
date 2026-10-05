#[cfg(feature = "alloc")]
use crate::atlas::{AtlasGlyph, GlyphAtlas};
#[cfg(feature = "alloc")]
use crate::blend::blend_glyph_mask;
use crate::blend::blend_pixel_premul;
use crate::damage::Rect;
use crate::font::{FONT_8X16, FONT_WIDTH};

/// Canvas drawing directly into a target CPU framebuffer (`&mut [u32]`).
pub struct Canvas<'a> {
    pub buffer: &'a mut [u32],
    pub width: u32,
    pub height: u32,
    pub clip: Rect,
}

impl<'a> Canvas<'a> {
    pub fn new(buffer: &'a mut [u32], width: u32, height: u32) -> Self {
        let clip = Rect::new(0, 0, width as i32, height as i32);
        Self {
            buffer,
            width,
            height,
            clip,
        }
    }

    /// Clears the entire buffer with a color.
    pub fn clear(&mut self, color: u32) {
        self.buffer.fill(color);
    }

    /// Fills a solid rectangle with alpha blending, constrained to the active clip rect.
    pub fn fill_rect(&mut self, rect: Rect, color: u32) {
        let visible = match rect.intersect(&self.clip) {
            Some(r) => r,
            None => return,
        };

        let x1 = visible.x.max(0) as u32;
        let y1 = visible.y.max(0) as u32;
        let x2 = (visible.right().max(0) as u32).min(self.width);
        let y2 = (visible.bottom().max(0) as u32).min(self.height);

        let fb_width = self.width as usize;

        for y in y1..y2 {
            let row_start = (y as usize) * fb_width;
            for x in x1..x2 {
                let idx = row_start + (x as usize);
                self.buffer[idx] = blend_pixel_premul(color, self.buffer[idx]);
            }
        }
    }

    /// Blits a glyph from the atlas into the framebuffer at (x, y) with a specified color.
    #[cfg(feature = "alloc")]
    pub fn blit_glyph(&mut self, atlas: &GlyphAtlas, glyph: &AtlasGlyph, x: i32, y: i32, color: u32) {
        let glyph_rect = Rect::new(x, y, glyph.width as i32, glyph.height as i32);
        let visible = match glyph_rect.intersect(&self.clip) {
            Some(r) => r,
            None => return,
        };

        let fb_width = self.width as usize;
        let atlas_width = atlas.width as usize;

        let gx_start = (visible.x - x) as usize;
        let gy_start = (visible.y - y) as usize;

        let x1 = visible.x as u32;
        let y1 = visible.y as u32;
        let w = visible.width as usize;
        let h = visible.height as usize;

        for row in 0..h {
            let atlas_row = ((glyph.y as usize) + gy_start + row) * atlas_width + (glyph.x as usize) + gx_start;
            let fb_row = ((y1 as usize) + row) * fb_width + (x1 as usize);

            for col in 0..w {
                let cov = atlas.pixels[atlas_row + col];
                self.buffer[fb_row + col] = blend_glyph_mask(color, cov, self.buffer[fb_row + col]);
            }
        }
    }

    /// Draws a single ASCII character (32..=126) using the embedded 8x16 bitmap font.
    pub fn draw_char(&mut self, c: char, x: i32, y: i32, color: u32) {
        let code = c as u32;
        if !(32..=127).contains(&code) {
            return;
        }
        let glyph_idx = (code - 32) as usize;
        let glyph_rows = &FONT_8X16[glyph_idx];

        for (row, &byte) in glyph_rows.iter().enumerate() {
            let py = y + row as i32;
            if py < self.clip.y || py >= self.clip.bottom() {
                continue;
            }
            for col in 0..8i32 {
                if (byte & (0x80 >> col)) != 0 {
                    let px = x + col;
                    if px >= self.clip.x && px < self.clip.right() {
                        let idx = (py as usize) * (self.width as usize) + (px as usize);
                        self.buffer[idx] = blend_pixel_premul(color, self.buffer[idx]);
                    }
                }
            }
        }
    }

    /// Draws a single line of ASCII text using the embedded 8x16 bitmap font.
    pub fn draw_text(&mut self, text: &str, mut x: i32, y: i32, color: u32) {
        for c in text.chars() {
            if c == '\n' {
                break;
            }
            self.draw_char(c, x, y, color);
            x += FONT_WIDTH;
        }
    }
}

