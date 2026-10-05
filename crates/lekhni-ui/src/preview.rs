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
        let mut curr_y = 10 - scroll_y; // 10px top margin
        let viewport_rect = Rect::new(0, 0, viewport_w, viewport_h);

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

            let block_rect = Rect::new(20, curr_y, viewport_w - 40, block_h);

            // Viewport culling: only draw if block intersects visible viewport
            if block_rect.intersect(&viewport_rect).is_some() {
                match kind {
                    BlockKind::Heading1 => {
                        // Title bar accent
                        canvas.fill_rect(Rect::new(20, curr_y + 32, viewport_w - 40, 2), 0xFF_4A_90_E2);
                    }
                    BlockKind::FencedCode => {
                        // Shaded code block background
                        canvas.fill_rect(block_rect, 0xFF_25_25_26);
                    }
                    BlockKind::BlockQuote => {
                        // Left vertical accent bar
                        canvas.fill_rect(Rect::new(20, curr_y, 4, block_h), 0xFF_00_7A_CC);
                    }
                    BlockKind::ThematicBreak => {
                        // Horizontal divider
                        canvas.fill_rect(Rect::new(20, curr_y + 5, viewport_w - 40, 1), 0xFF_55_55_55);
                    }
                    _ => {}
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
