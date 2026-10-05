//! Viewport-only live Markdown preview renderer.
//!
//! Follows Section 5 of LEKHNI_ARCHITECTURE:
//! Only processes and renders blocks visible within the viewport bounding box.

use lekhni_md::block::{BlockKind, BlockTable};
use lekhni_raster::canvas::Canvas;
use lekhni_raster::damage::Rect;

/// Viewport-only live Markdown preview widget renderer.
pub struct PreviewRenderer;

impl PreviewRenderer {
    /// Renders visible Markdown blocks into the canvas framebuffer.
    ///
    /// Only blocks whose vertical layout bounds intersect `(scroll_y..scroll_y + viewport_h)`
    /// will be drawn.
    pub fn render_preview(
        canvas: &mut Canvas,
        blocks: &BlockTable,
        scroll_y: i32,
        viewport_w: i32,
        viewport_h: i32,
    ) {
        Self::render_preview_with_content(canvas, blocks, &[], scroll_y, viewport_w, viewport_h);
    }

    /// Renders visible Markdown blocks along with their text content into the canvas framebuffer.
    pub fn render_preview_with_content(
        canvas: &mut Canvas,
        blocks: &BlockTable,
        doc_bytes: &[u8],
        scroll_y: i32,
        viewport_w: i32,
        viewport_h: i32,
    ) {
        let origin_x = canvas.clip.x;
        let origin_y = canvas.clip.y;
        let mut curr_y = origin_y + 10 - scroll_y; // 10px top margin
        let viewport_rect = Rect::new(origin_x, origin_y, viewport_w, viewport_h);

        for i in 0..blocks.len() {
            let kind = BlockKind::from_u8(blocks.kind[i]);
            let block_h = match kind {
                BlockKind::Heading1 => 36,
                BlockKind::Heading2 => 30,
                BlockKind::Heading3 => 26,
                BlockKind::FencedCode => 48,
                BlockKind::BlockQuote => 28,
                BlockKind::ThematicBreak => 12,
                _ => 22,
            };

            let block_rect = Rect::new(origin_x + 20, curr_y, viewport_w - 40, block_h);

            // Viewport culling: only draw if block intersects visible viewport
            if block_rect.intersect(&viewport_rect).is_some() {
                let start = (blocks.start[i] as usize).min(doc_bytes.len());
                let end = (blocks.end[i] as usize).min(doc_bytes.len());
                let text_slice = if start < end { &doc_bytes[start..end] } else { b"" };
                let text_str = core::str::from_utf8(text_slice).unwrap_or("");
                let clean_text = text_str.lines().next().unwrap_or("").trim();

                match kind {
                    BlockKind::Heading1 => {
                        canvas.fill_rect(Rect::new(origin_x + 20, curr_y + 32, viewport_w - 40, 2), 0xFF_4A_90_E2);
                        let heading_label = clean_text.trim_start_matches('#').trim();
                        canvas.draw_text(heading_label, origin_x + 20, curr_y + 10, 0xFF_FF_FF_FF);
                    }
                    BlockKind::Heading2 => {
                        let heading_label = clean_text.trim_start_matches('#').trim();
                        canvas.draw_text(heading_label, origin_x + 20, curr_y + 6, 0xFF_E0_E0_E0);
                    }
                    BlockKind::Heading3 => {
                        let heading_label = clean_text.trim_start_matches('#').trim();
                        canvas.draw_text(heading_label, origin_x + 20, curr_y + 4, 0xFF_C5_C5_C5);
                    }
                    BlockKind::FencedCode => {
                        canvas.fill_rect(block_rect, 0xFF_25_25_26);
                        let code_content = clean_text.trim_start_matches('`').trim();
                        canvas.draw_text(code_content, origin_x + 28, curr_y + 16, 0xFF_CE_91_78);
                    }
                    BlockKind::BlockQuote => {
                        canvas.fill_rect(Rect::new(origin_x + 20, curr_y, 4, block_h), 0xFF_00_7A_CC);
                        let quote_text = clean_text.trim_start_matches('>').trim();
                        canvas.draw_text(quote_text, origin_x + 32, curr_y + 6, 0xFF_9C_DC_FE);
                    }
                    BlockKind::ThematicBreak => {
                        canvas.fill_rect(Rect::new(origin_x + 20, curr_y + 5, viewport_w - 40, 1), 0xFF_55_55_55);
                    }
                    BlockKind::ListItem => {
                        canvas.draw_text(clean_text, origin_x + 24, curr_y + 3, 0xFF_B5_CE_A8);
                    }
                    _ => {
                        canvas.draw_text(clean_text, origin_x + 20, curr_y + 3, 0xFF_D4_D4_D4);
                    }
                }
            }

            curr_y += block_h + 8; // Block height + margin

            // Early exit if past bottom of viewport
            if curr_y > viewport_h {
                break;
            }
        }
    }
}
