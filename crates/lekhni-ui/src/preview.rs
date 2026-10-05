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

    /// Renders visible Markdown blocks along with their rich formatted text content.
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
        let mut curr_y = origin_y + 12 - scroll_y;
        let viewport_rect = Rect::new(origin_x, origin_y, viewport_w, viewport_h);

        for i in 0..blocks.len() {
            let kind = BlockKind::from_u8(blocks.kind[i]);
            let block_h = match kind {
                BlockKind::Heading1 => 44,
                BlockKind::Heading2 => 32,
                BlockKind::Heading3 => 26,
                BlockKind::FencedCode => 54,
                BlockKind::BlockQuote => 28,
                BlockKind::ThematicBreak => 16,
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
                        let heading_label = clean_text.trim_start_matches('#').trim();
                        // 2x Scaled Heading 1 font
                        canvas.draw_text_scaled(heading_label, origin_x + 20, curr_y + 4, 0xFF_FF_FF_FF, 2);
                        canvas.fill_rect(Rect::new(origin_x + 20, curr_y + 38, viewport_w - 40, 2), 0xFF_4A_90_E2);
                    }
                    BlockKind::Heading2 => {
                        let heading_label = clean_text.trim_start_matches('#').trim();
                        canvas.draw_text(heading_label, origin_x + 20, curr_y + 6, 0xFF_FF_FF_FF);
                        canvas.draw_text(heading_label, origin_x + 21, curr_y + 6, 0xFF_FF_FF_FF); // bold
                        canvas.fill_rect(Rect::new(origin_x + 20, curr_y + 26, viewport_w - 40, 1), 0xFF_3E_44_51);
                    }
                    BlockKind::Heading3 => {
                        let heading_label = clean_text.trim_start_matches('#').trim();
                        canvas.draw_text(heading_label, origin_x + 20, curr_y + 4, 0xFF_4E_C9_B0);
                    }
                    BlockKind::FencedCode => {
                        canvas.fill_rect(block_rect, 0xFF_21_25_2B);
                        let code_content = clean_text.trim_start_matches('`').trim();
                        canvas.draw_text(code_content, origin_x + 28, curr_y + 18, 0xFF_98_C3_79);
                    }
                    BlockKind::BlockQuote => {
                        canvas.fill_rect(Rect::new(origin_x + 20, curr_y, 4, block_h), 0xFF_00_7A_CC);
                        let quote_text = clean_text.trim_start_matches('>').trim();
                        render_inline_markdown(canvas, quote_text, origin_x + 32, curr_y + 6, 0xFF_9C_DC_FE);
                    }
                    BlockKind::ThematicBreak => {
                        canvas.fill_rect(Rect::new(origin_x + 20, curr_y + 8, viewport_w - 40, 1), 0xFF_4B_52_63);
                    }
                    BlockKind::ListItem => {
                        if let Some(rest) = clean_text.strip_prefix("- [ ] ") {
                            canvas.fill_rect(Rect::new(origin_x + 20, curr_y + 4, 12, 12), 0xFF_3E_44_51);
                            canvas.fill_rect(Rect::new(origin_x + 21, curr_y + 5, 10, 10), 0xFF_28_2C_34);
                            render_inline_markdown(canvas, rest, origin_x + 38, curr_y + 3, 0xFF_AB_B2_BF);
                        } else if let Some(rest) = clean_text.strip_prefix("- [x] ") {
                            canvas.fill_rect(Rect::new(origin_x + 20, curr_y + 4, 12, 12), 0xFF_98_C3_79);
                            canvas.draw_text("v", origin_x + 22, curr_y + 3, 0xFF_1E_1E_1E);
                            render_inline_markdown(canvas, rest, origin_x + 38, curr_y + 3, 0xFF_98_C3_79);
                        } else {
                            canvas.draw_char('*', origin_x + 20, curr_y + 3, 0xFF_61_AF_EF);
                            let rest = clean_text.trim_start_matches(&['*', '-'][..]).trim_start();
                            render_inline_markdown(canvas, rest, origin_x + 32, curr_y + 3, 0xFF_D4_D4_D4);
                        }
                    }
                    _ => {
                        render_inline_markdown(canvas, clean_text, origin_x + 20, curr_y + 3, 0xFF_D4_D4_D4);
                    }
                }
            }

            curr_y += block_h + 8; // Block height + margin

            if curr_y > viewport_h {
                break;
            }
        }
    }
}

/// Renders an inline markdown string with formatting (bold, italic, inline code, and links).
fn render_inline_markdown(canvas: &mut Canvas, line: &str, mut x: i32, y: i32, default_color: u32) {
    let bytes = line.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if i + 1 < bytes.len() && bytes[i] == b'*' && bytes[i + 1] == b'*' {
            if let Some(end) = line[i + 2..].find("**") {
                let bold_text = &line[i + 2..i + 2 + end];
                canvas.draw_text(bold_text, x, y, 0xFF_FF_FF_FF);
                canvas.draw_text(bold_text, x + 1, y, 0xFF_FF_FF_FF);
                x += (bold_text.len() as i32) * 8;
                i += 4 + end;
                continue;
            }
        }
        if bytes[i] == b'`' {
            if let Some(end) = line[i + 1..].find('`') {
                let code_text = &line[i + 1..i + 1 + end];
                let w = (code_text.len() as i32) * 8 + 6;
                canvas.fill_rect(Rect::new(x, y - 1, w, 18), 0xFF_2D_31_39);
                canvas.draw_text(code_text, x + 3, y, 0xFF_E0_6C_75);
                x += w + 2;
                i += 2 + end;
                continue;
            }
        }
        if bytes[i] == b'[' {
            if let Some(close_bracket) = line[i + 1..].find(']') {
                let rest = &line[i + 1 + close_bracket + 1..];
                if rest.starts_with('(') {
                    if let Some(close_paren) = rest.find(')') {
                        let link_label = &line[i + 1..i + 1 + close_bracket];
                        canvas.draw_text(link_label, x, y, 0xFF_61_AF_EF);
                        let w = (link_label.len() as i32) * 8;
                        canvas.fill_rect(Rect::new(x, y + 15, w, 1), 0xFF_61_AF_EF);
                        x += w;
                        i += 1 + close_bracket + 1 + close_paren + 1;
                        continue;
                    }
                }
            }
        }
        if bytes[i] == b'*' {
            if let Some(end) = line[i + 1..].find('*') {
                let italic_text = &line[i + 1..i + 1 + end];
                canvas.draw_text(italic_text, x, y, 0xFF_E5_C0_7B);
                x += (italic_text.len() as i32) * 8;
                i += 2 + end;
                continue;
            }
        }

        let ch = line[i..].chars().next().unwrap_or(' ');
        canvas.draw_char(ch, x, y, default_color);
        x += 8;
        i += ch.len_utf8();
    }
}

