//! Vector font rasterization and L1 cache-resident glyph atlas engine.
//!
//! Powered by pure Rust no_std `fontdue` without C dependencies.
//! Pre-rasterizes ASCII (32..=126) into contiguous L1-resident array tables.

use crate::atlas::{AtlasGlyph, GlyphAtlas};
use crate::canvas::Canvas;

pub static INTER_REGULAR: &[u8] = include_bytes!("../fonts/Inter-Regular.otf");
pub static GO_MONO: &[u8] = include_bytes!("../fonts/Go-Mono.ttf");

/// High-performance cache-resident vector font with pre-rasterized glyphs.
#[cfg(feature = "alloc")]
pub struct CachedFont {
    pub atlas: GlyphAtlas,
    pub glyphs: [AtlasGlyph; 128],
    pub size_px: f32,
    pub line_height: i32,
    pub ascent: i32,
}

#[cfg(feature = "alloc")]
impl CachedFont {
    /// Creates and pre-rasterizes ASCII glyphs from TrueType/OpenType font data.
    pub fn from_bytes(font_bytes: &[u8], size_px: f32) -> Result<Self, &'static str> {
        let font = fontdue::Font::from_bytes(font_bytes, fontdue::FontSettings::default())
            .map_err(|_| "Failed to parse vector font bytes")?;

        let line_metrics = font.horizontal_line_metrics(size_px)
            .unwrap_or(fontdue::LineMetrics {
                ascent: size_px * 0.8,
                descent: -size_px * 0.2,
                line_gap: size_px * 0.2,
                new_line_size: size_px * 1.2,
            });

        let ascent = line_metrics.ascent.round() as i32;
        let line_height = (line_metrics.ascent - line_metrics.descent + line_metrics.line_gap).round() as i32;

        // Shelf atlas size: 512x512 easily houses 128 ASCII glyphs for sizes up to 36px
        let mut atlas = GlyphAtlas::new(512, 512);
        let mut glyphs = [AtlasGlyph::default(); 128];

        for c_val in 0..128u8 {
            let ch = c_val as char;
            if (32..=126).contains(&c_val) {
                let (metrics, bitmap) = font.rasterize(ch, size_px);
                let w = metrics.width as u32;
                let h = metrics.height as u32;

                if w > 0 && h > 0 {
                    if let Some(slot) = atlas.allocate(w, h) {
                        atlas.write_glyph(&slot, &bitmap);
                        // Vertical positioning: top of glyph on canvas relative to text baseline
                        let offset_y = ascent - (metrics.ymin + metrics.height as i32);
                        glyphs[c_val as usize] = AtlasGlyph {
                            x: slot.x,
                            y: slot.y,
                            width: slot.width,
                            height: slot.height,
                            offset_x: metrics.xmin as i16,
                            offset_y: offset_y as i16,
                            advance_x: metrics.advance_width.round().max(1.0) as u16,
                        };
                    } else {
                        // If atlas overflowed, fall back to basic metrics
                        glyphs[c_val as usize].advance_x = metrics.advance_width.round().max(1.0) as u16;
                    }
                } else {
                    // Whitespace (e.g. space)
                    glyphs[c_val as usize] = AtlasGlyph {
                        x: 0,
                        y: 0,
                        width: 0,
                        height: 0,
                        offset_x: 0,
                        offset_y: 0,
                        advance_x: metrics.advance_width.round().max(1.0) as u16,
                    };
                }
            } else {
                // Non-printable control characters
                glyphs[c_val as usize] = AtlasGlyph::default();
            }
        }

        Ok(Self {
            atlas,
            glyphs,
            size_px,
            line_height: line_height.max(16),
            ascent,
        })
    }

    /// Measures single-line text width and height with zero allocations.
    #[inline(always)]
    pub fn measure_text(&self, text: &str) -> (i32, i32) {
        let mut width = 0i32;
        for b in text.bytes() {
            if b == b'\n' {
                break;
            }
            let idx = (b as usize).min(127);
            width += self.glyphs[idx].advance_x as i32;
        }
        (width, self.line_height)
    }

    /// Draws anti-aliased text into canvas and returns the total advanced width.
    pub fn draw_text(&self, canvas: &mut Canvas, text: &str, mut x: i32, y: i32, color: u32) -> i32 {
        let start_x = x;
        for b in text.bytes() {
            if b == b'\n' {
                break;
            }
            let idx = (b as usize).min(127);
            let g = &self.glyphs[idx];

            if g.width > 0 && g.height > 0 {
                let gx = x + g.offset_x as i32;
                let gy = y + g.offset_y as i32;
                canvas.blit_glyph(&self.atlas, g, gx, gy, color);
            }

            x += g.advance_x as i32;
        }
        x - start_x
    }
}

/// Pre-allocated collection of modern typography styles for UI, Editor, and Preview.
#[cfg(feature = "alloc")]
pub struct FontCollection {
    pub ui: CachedFont,
    pub ui_bold: CachedFont,
    pub editor: CachedFont,
    pub body: CachedFont,
    pub h1: CachedFont,
    pub h2: CachedFont,
    pub h3: CachedFont,
}

#[cfg(feature = "alloc")]
impl FontCollection {
    /// Loads default modern typography set.
    pub fn load_default() -> Result<Self, &'static str> {
        let ui = CachedFont::from_bytes(INTER_REGULAR, 14.0)?;
        let ui_bold = CachedFont::from_bytes(INTER_REGULAR, 14.0)?;
        let editor = CachedFont::from_bytes(GO_MONO, 14.5)?;
        let body = CachedFont::from_bytes(INTER_REGULAR, 15.0)?;
        let h1 = CachedFont::from_bytes(INTER_REGULAR, 24.0)?;
        let h2 = CachedFont::from_bytes(INTER_REGULAR, 19.0)?;
        let h3 = CachedFont::from_bytes(INTER_REGULAR, 16.0)?;

        Ok(Self {
            ui,
            ui_bold,
            editor,
            body,
            h1,
            h2,
            h3,
        })
    }
}
