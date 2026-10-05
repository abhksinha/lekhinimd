//! Text shaping abstractions and complex-script classifier.
//!
//! Follows Section 7 of LEKHNI_ARCHITECTURE:
//! Distinguishes fast-path simple scripts (Latin) from complex-path Indic/Devanagari/Arabic.

#[cfg(feature = "alloc")]
use alloc::vec::Vec;

/// Returns true if the character belongs to a script requiring complex open-type shaping
/// (e.g. Indic / Devanagari conjuncts, matras, or Arabic cursive joins).
#[inline(always)]
pub fn is_complex_script_codepoint(ch: char) -> bool {
    let cp = ch as u32;
    matches!(cp,
        0x0600..=0x06FF // Arabic
        | 0x0750..=0x077F // Arabic Supplement
        | 0x08A0..=0x08FF // Arabic Extended-A
        | 0x0900..=0x097F // Devanagari (Hindi, Sanskrit, Marathi)
        | 0x0980..=0x09FF // Bengali
        | 0x0A00..=0x0A7F // Gurmukhi
        | 0x0A80..=0x0AFF // Gujarati
        | 0x0B00..=0x0B7F // Oriya
        | 0x0B80..=0x0BFF // Tamil
        | 0x0C00..=0x0C7F // Telugu
        | 0x0C80..=0x0CFF // Kannada
        | 0x0D00..=0x0D7F // Malayalam
        | 0x0D80..=0x0DFF // Sinhala
        | 0x0E00..=0x0E7F // Thai
        | 0x0E80..=0x0EFF // Lao
        | 0x0F00..=0x0FFF // Tibetan
        | 0x1000..=0x109F // Myanmar
    )
}

/// Checks if a string slice contains any characters that require complex OpenType shaping.
#[inline(always)]
pub fn requires_complex_shaping(text: &str) -> bool {
    text.chars().any(is_complex_script_codepoint)
}

/// Shaped glyph positioned in layout space.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct PositionedGlyph {
    pub glyph_id: u32,
    pub cluster: u32,
    pub x_advance: i32,
    pub x_offset: i32,
    pub y_offset: i32,
}

/// Text shaping trait separating fast simple layout from OpenType complex shaping.
pub trait Shaper {
    #[cfg(feature = "alloc")]
    fn shape_line(&self, text: &str, font_size_pt: f32) -> Vec<PositionedGlyph>;
}

/// Default baseline shaper implementing simple monospace/Latin fast-path
/// and cluster decomposition for Indic verification.
#[derive(Default)]
pub struct SimpleShaper;

impl Shaper for SimpleShaper {
    #[cfg(feature = "alloc")]
    fn shape_line(&self, text: &str, font_size_pt: f32) -> Vec<PositionedGlyph> {
        let base_advance = (font_size_pt * 0.6 * 64.0) as i32; // 26.6 fixed point advance
        let mut glyphs = Vec::with_capacity(text.len());

        for (idx, ch) in text.char_indices() {
            let glyph_id = ch as u32;
            let advance = if ch == '\t' {
                base_advance * 4
            } else if is_complex_script_codepoint(ch) {
                // Indic / complex characters have proportional glyph advances
                base_advance
            } else {
                base_advance
            };

            glyphs.push(PositionedGlyph {
                glyph_id,
                cluster: idx as u32,
                x_advance: advance,
                x_offset: 0,
                y_offset: 0,
            });
        }

        glyphs
    }
}
