//! Viewport-only live Markdown preview renderer.
//!
//! Follows Section 5 of LEKHNI_ARCHITECTURE:
//! - Full multi-line layout and height measurement for every block.
//! - Rich inline formatting: bold, italic, code badges, strikethrough, links.
//! - Fenced code blocks, simple tables, and relative images.
//! - Interactive clickable target bounding boxes (links, task checkboxes, headings).

#[cfg(feature = "alloc")]
use alloc::format;
#[cfg(feature = "alloc")]
use alloc::string::{String, ToString};
#[cfg(feature = "alloc")]
use alloc::vec::Vec;

use lekhni_md::block::{BlockKind, BlockTable};
use lekhni_raster::canvas::Canvas;
use lekhni_raster::damage::Rect;
use lekhni_raster::vector_font::{CachedFont, FontCollection};

/// Interactive clickable target detected during preview rendering.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ClickableTarget {
    Link { rect: Rect, url: String },
    Checkbox { rect: Rect, source_byte_offset: usize, is_checked: bool },
    Heading { rect: Rect, source_byte_offset: usize },
}

/// Precomputed layout bounds for a Markdown block.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct BlockLayout {
    pub y: i32,
    pub height: i32,
    pub source_start: usize,
    pub source_end: usize,
}

/// Viewport live Markdown preview widget renderer.
pub struct PreviewRenderer;

impl PreviewRenderer {
    /// Renders visible Markdown blocks into the canvas framebuffer.
    pub fn render_preview(
        canvas: &mut Canvas,
        blocks: &BlockTable,
        scroll_y: i32,
        viewport_w: i32,
        viewport_h: i32,
    ) {
        let _ = Self::render_preview_with_content(canvas, blocks, &[], scroll_y, viewport_w, viewport_h);
    }

    /// Computes full visual layout and heights for all blocks in the document.
    #[cfg(feature = "alloc")]
    pub fn compute_layouts(
        blocks: &BlockTable,
        doc_bytes: &[u8],
        viewport_w: i32,
    ) -> Vec<BlockLayout> {
        let mut layouts = Vec::with_capacity(blocks.len());
        let mut curr_y = 12;

        for i in 0..blocks.len() {
            let kind = BlockKind::from_u8(blocks.kind[i]);
            let start = (blocks.start[i] as usize).min(doc_bytes.len());
            let end = (blocks.end[i] as usize).min(doc_bytes.len());
            let block_bytes = if start < end { &doc_bytes[start..end] } else { b"" };
            let block_str = core::str::from_utf8(block_bytes).unwrap_or("");

            let block_h = match kind {
                BlockKind::Heading1 => 44,
                BlockKind::Heading2 => 34,
                BlockKind::Heading3 => 28,
                BlockKind::Heading4 | BlockKind::Heading5 | BlockKind::Heading6 => 24,
                BlockKind::ThematicBreak => 16,
                BlockKind::FencedCode => {
                    let total_lines = block_str.lines().count();
                    let code_lines = total_lines.saturating_sub(2).max(1);
                    24 + (code_lines as i32) * 18 + 8
                }
                BlockKind::Table => {
                    let rows = block_str.lines().filter(|l| {
                        let t = l.trim();
                        !t.is_empty() && !is_table_separator(t)
                    }).count().max(1);
                    10 + (rows as i32) * 24 + 6
                }
                BlockKind::BlockQuote => {
                    let max_c = ((viewport_w - 60) / 8).max(10) as usize;
                    let mut visual_lines = 0;
                    for l in block_str.lines() {
                        let clean = l.trim_start().trim_start_matches('>').trim();
                        visual_lines += count_wrapped_lines(clean, max_c).max(1);
                    }
                    (visual_lines.max(1) as i32) * 18 + 8
                }
                BlockKind::ListItem => {
                    let max_c = ((viewport_w - 60) / 8).max(10) as usize;
                    let mut visual_lines = 0;
                    for l in block_str.lines() {
                        let clean = l.trim();
                        let content = if let Some(r) = clean.strip_prefix("- [ ] ").or_else(|| clean.strip_prefix("- [x] ")) {
                            r
                        } else {
                            clean.trim_start_matches(&['*', '-', '+'][..]).trim_start()
                        };
                        visual_lines += count_wrapped_lines(content, max_c).max(1);
                    }
                    (visual_lines.max(1) as i32) * 18 + 6
                }
                _ => { // Paragraph
                    let max_c = ((viewport_w - 40) / 8).max(10) as usize;
                    let mut visual_lines = 0;
                    let mut is_img = false;
                    for l in block_str.lines() {
                        let clean = l.trim();
                        if clean.starts_with("![") && clean.contains("](") && clean.ends_with(')') {
                            is_img = true;
                            break;
                        }
                        visual_lines += count_wrapped_lines(clean, max_c).max(1);
                    }
                    if is_img {
                        54
                    } else {
                        (visual_lines.max(1) as i32) * 18 + 8
                    }
                }
            };

            layouts.push(BlockLayout {
                y: curr_y,
                height: block_h,
                source_start: start,
                source_end: end,
            });

            curr_y += block_h + 8; // Margin between blocks
        }

        layouts
    }

    /// Renders visible Markdown blocks along with their rich formatted text content.
    ///
    /// Returns:
    /// - Vector of interactive clickable targets (links, task checkboxes, headings).
    #[cfg(feature = "alloc")]
    pub fn render_preview_with_content(
        canvas: &mut Canvas,
        blocks: &BlockTable,
        doc_bytes: &[u8],
        scroll_y: i32,
        viewport_w: i32,
        viewport_h: i32,
    ) -> Vec<ClickableTarget> {
        Self::render_preview_with_fonts(canvas, blocks, doc_bytes, scroll_y, viewport_w, viewport_h, None)
    }

    /// Renders visible Markdown blocks along with their rich formatted text content using optional modern vector fonts.
    #[cfg(feature = "alloc")]
    pub fn render_preview_with_fonts(
        canvas: &mut Canvas,
        blocks: &BlockTable,
        doc_bytes: &[u8],
        scroll_y: i32,
        viewport_w: i32,
        viewport_h: i32,
        fonts: Option<&FontCollection>,
    ) -> Vec<ClickableTarget> {
        let origin_x = canvas.clip.x;
        let origin_y = canvas.clip.y;
        let viewport_rect = Rect::new(origin_x, origin_y, viewport_w, viewport_h);
        let mut click_targets = Vec::new();

        let layouts = Self::compute_layouts(blocks, doc_bytes, viewport_w);

        for (i, layout) in layouts.iter().enumerate() {
            let screen_y = origin_y + layout.y - scroll_y;
            let block_rect = Rect::new(origin_x + 20, screen_y, viewport_w - 40, layout.height);

            // Viewport culling: only draw if block intersects visible screen viewport
            if block_rect.intersect(&viewport_rect).is_none() {
                continue;
            }

            let kind = BlockKind::from_u8(blocks.kind[i]);
            let block_bytes = if layout.source_start < layout.source_end {
                &doc_bytes[layout.source_start..layout.source_end]
            } else {
                b""
            };
            let block_str = core::str::from_utf8(block_bytes).unwrap_or("");

            match kind {
                BlockKind::Heading1 => {
                    let clean = block_str.lines().next().unwrap_or("").trim();
                    let heading_label = clean.trim_start_matches('#').trim();
                    if let Some(f) = fonts {
                        f.h1.draw_text(canvas, heading_label, origin_x + 20, screen_y + 4, 0xFF_FF_FF_FF);
                    } else {
                        // 2x Scaled Heading 1 font
                        canvas.draw_text_scaled(heading_label, origin_x + 20, screen_y + 4, 0xFF_FF_FF_FF, 2);
                    }
                    canvas.fill_rect(Rect::new(origin_x + 20, screen_y + 38, viewport_w - 40, 2), 0xFF_4A_90_E2);
                    click_targets.push(ClickableTarget::Heading {
                        rect: block_rect,
                        source_byte_offset: layout.source_start,
                    });
                }
                BlockKind::Heading2 => {
                    let clean = block_str.lines().next().unwrap_or("").trim();
                    let heading_label = clean.trim_start_matches('#').trim();
                    if let Some(f) = fonts {
                        f.h2.draw_text(canvas, heading_label, origin_x + 20, screen_y + 6, 0xFF_FF_FF_FF);
                    } else {
                        canvas.draw_text(heading_label, origin_x + 20, screen_y + 6, 0xFF_FF_FF_FF);
                        canvas.draw_text(heading_label, origin_x + 21, screen_y + 6, 0xFF_FF_FF_FF); // bold
                    }
                    canvas.fill_rect(Rect::new(origin_x + 20, screen_y + 28, viewport_w - 40, 1), 0xFF_3E_44_51);
                    click_targets.push(ClickableTarget::Heading {
                        rect: block_rect,
                        source_byte_offset: layout.source_start,
                    });
                }
                BlockKind::Heading3 => {
                    let clean = block_str.lines().next().unwrap_or("").trim();
                    let heading_label = clean.trim_start_matches('#').trim();
                    if let Some(f) = fonts {
                        f.h3.draw_text(canvas, heading_label, origin_x + 20, screen_y + 4, 0xFF_4E_C9_B0);
                    } else {
                        canvas.draw_text(heading_label, origin_x + 20, screen_y + 4, 0xFF_4E_C9_B0);
                    }
                    click_targets.push(ClickableTarget::Heading {
                        rect: block_rect,
                        source_byte_offset: layout.source_start,
                    });
                }
                BlockKind::Heading4 | BlockKind::Heading5 | BlockKind::Heading6 => {
                    let clean = block_str.lines().next().unwrap_or("").trim();
                    let heading_label = clean.trim_start_matches('#').trim();
                    if let Some(f) = fonts {
                        f.body.draw_text(canvas, heading_label, origin_x + 20, screen_y + 4, 0xFF_E5_C0_7B);
                    } else {
                        canvas.draw_text(heading_label, origin_x + 20, screen_y + 4, 0xFF_E5_C0_7B);
                    }
                    click_targets.push(ClickableTarget::Heading {
                        rect: block_rect,
                        source_byte_offset: layout.source_start,
                    });
                }
                BlockKind::FencedCode => {
                    // Background and border
                    canvas.fill_rect(block_rect, 0xFF_21_25_2B);
                    canvas.fill_rect(Rect::new(block_rect.x, block_rect.y, block_rect.width, 1), 0xFF_3E_44_51);
                    canvas.fill_rect(Rect::new(block_rect.x, block_rect.bottom() - 1, block_rect.width, 1), 0xFF_3E_44_51);
                    canvas.fill_rect(Rect::new(block_rect.x, block_rect.y, 1, block_rect.height), 0xFF_3E_44_51);
                    canvas.fill_rect(Rect::new(block_rect.right() - 1, block_rect.y, 1, block_rect.height), 0xFF_3E_44_51);

                    let mut lines = block_str.lines();
                    let first_line = lines.next().unwrap_or("");
                    let lang_tag = first_line.trim_start_matches(&['`', '~'][..]).trim();
                    let lang_display = if lang_tag.is_empty() { "CODE" } else { lang_tag };

                    // Header bar
                    canvas.fill_rect(Rect::new(block_rect.x + 1, block_rect.y + 1, block_rect.width - 2, 20), 0xFF_1E_22_27);
                    if let Some(f) = fonts {
                        let (tw, _) = f.ui.measure_text(lang_display);
                        f.ui.draw_text(canvas, lang_display, block_rect.right() - tw - 12, screen_y + 4, 0xFF_5C_63_70);
                    } else {
                        canvas.draw_text(lang_display, block_rect.right() - (lang_display.len() as i32 * 8) - 12, screen_y + 4, 0xFF_5C_63_70);
                    }

                    let mut code_y = screen_y + 24;
                    let mut code_line_num = 1;
                    for line in lines {
                        let t = line.trim_end();
                        if t.starts_with("```") || t.starts_with("~~~") {
                            break;
                        }
                        if code_y + 18 <= block_rect.bottom() {
                            let num_str = format!("{:2}", code_line_num);
                            if let Some(f) = fonts {
                                f.editor.draw_text(canvas, &num_str, block_rect.x + 8, code_y, 0xFF_4B_52_63);
                                f.editor.draw_text(canvas, line, block_rect.x + 34, code_y, 0xFF_98_C3_79);
                            } else {
                                canvas.draw_text(&num_str, block_rect.x + 8, code_y, 0xFF_4B_52_63);
                                canvas.draw_text(line, block_rect.x + 34, code_y, 0xFF_98_C3_79);
                            }
                        }
                        code_y += 18;
                        code_line_num += 1;
                    }
                }
                BlockKind::Table => {
                    let mut table_y = screen_y + 6;
                    let mut is_header = true;
                    for line in block_str.lines() {
                        let t = line.trim();
                        if t.is_empty() {
                            continue;
                        }
                        if is_table_separator(t) {
                            continue;
                        }

                        let cells: Vec<&str> = t.split('|')
                            .map(|c| c.trim())
                            .filter(|c| !c.is_empty())
                            .collect();

                        if cells.is_empty() {
                            continue;
                        }

                        let col_w = (viewport_w - 50) / (cells.len().max(1) as i32);
                        let row_rect = Rect::new(origin_x + 20, table_y, viewport_w - 40, 22);

                        if is_header {
                            canvas.fill_rect(row_rect, 0xFF_2C_31_3A);
                            canvas.fill_rect(Rect::new(row_rect.x, row_rect.bottom() - 1, row_rect.width, 1), 0xFF_61_AF_EF);
                            for (c_idx, cell) in cells.iter().enumerate() {
                                let cx = origin_x + 24 + (c_idx as i32) * col_w;
                                if let Some(f) = fonts {
                                    f.ui_bold.draw_text(canvas, cell, cx, table_y + 3, 0xFF_61_AF_EF);
                                } else {
                                    canvas.draw_text(cell, cx, table_y + 3, 0xFF_61_AF_EF);
                                    canvas.draw_text(cell, cx + 1, table_y + 3, 0xFF_61_AF_EF); // bold
                                }
                            }
                            is_header = false;
                        } else {
                            canvas.fill_rect(row_rect, 0xFF_21_25_2B);
                            canvas.fill_rect(Rect::new(row_rect.x, row_rect.bottom() - 1, row_rect.width, 1), 0xFF_2D_31_39);
                            for (c_idx, cell) in cells.iter().enumerate() {
                                let cx = origin_x + 24 + (c_idx as i32) * col_w;
                                if let Some(f) = fonts {
                                    f.ui.draw_text(canvas, cell, cx, table_y + 3, 0xFF_AB_B2_BF);
                                } else {
                                    canvas.draw_text(cell, cx, table_y + 3, 0xFF_AB_B2_BF);
                                }
                            }
                        }

                        table_y += 24;
                    }
                }
                BlockKind::BlockQuote => {
                    canvas.fill_rect(Rect::new(origin_x + 20, screen_y, 4, layout.height), 0xFF_00_7A_CC);
                    let mut quote_y = screen_y + 4;
                    let max_c = ((viewport_w - 60) / 8).max(10) as usize;
                    let font_body = fonts.map(|f| &f.body);
                    for l in block_str.lines() {
                        let clean = l.trim_start().trim_start_matches('>').trim();
                        wrap_and_render_inline(canvas, clean, origin_x + 32, &mut quote_y, max_c, 0xFF_9C_DC_FE, font_body, &mut click_targets);
                    }
                }
                BlockKind::ThematicBreak => {
                    canvas.fill_rect(Rect::new(origin_x + 20, screen_y + 8, viewport_w - 40, 1), 0xFF_4B_52_63);
                }
                BlockKind::ListItem => {
                    let mut list_y = screen_y + 3;
                    let max_c = ((viewport_w - 60) / 8).max(10) as usize;
                    let mut line_offset_acc = layout.source_start;
                    let font_body = fonts.map(|f| &f.body);

                    for l in block_str.lines() {
                        let clean = l.trim();
                        let line_len = l.len();

                        if let Some(rest) = clean.strip_prefix("- [ ] ") {
                            let check_rect = Rect::new(origin_x + 20, list_y + 2, 14, 14);
                            canvas.fill_rect(check_rect, 0xFF_3E_44_51);
                            canvas.fill_rect(Rect::new(origin_x + 21, list_y + 3, 12, 12), 0xFF_21_25_2B);
                            click_targets.push(ClickableTarget::Checkbox {
                                rect: check_rect,
                                source_byte_offset: line_offset_acc,
                                is_checked: false,
                            });
                            wrap_and_render_inline(canvas, rest, origin_x + 40, &mut list_y, max_c, 0xFF_AB_B2_BF, font_body, &mut click_targets);
                        } else if let Some(rest) = clean.strip_prefix("- [x] ") {
                            let check_rect = Rect::new(origin_x + 20, list_y + 2, 14, 14);
                            canvas.fill_rect(check_rect, 0xFF_98_C3_79);
                            if let Some(f) = fonts {
                                f.ui_bold.draw_text(canvas, "v", origin_x + 23, list_y + 1, 0xFF_1E_1E_1E);
                            } else {
                                canvas.draw_text("v", origin_x + 23, list_y + 1, 0xFF_1E_1E_1E);
                            }
                            click_targets.push(ClickableTarget::Checkbox {
                                rect: check_rect,
                                source_byte_offset: line_offset_acc,
                                is_checked: true,
                            });
                            wrap_and_render_inline(canvas, rest, origin_x + 40, &mut list_y, max_c, 0xFF_98_C3_79, font_body, &mut click_targets);
                        } else {
                            if let Some(f) = fonts {
                                f.ui_bold.draw_text(canvas, "*", origin_x + 20, list_y + 1, 0xFF_61_AF_EF);
                            } else {
                                canvas.draw_char('*', origin_x + 20, list_y + 1, 0xFF_61_AF_EF);
                            }
                            let rest = clean.trim_start_matches(&['*', '-', '+'][..]).trim_start();
                            wrap_and_render_inline(canvas, rest, origin_x + 34, &mut list_y, max_c, 0xFF_D4_D4_D4, font_body, &mut click_targets);
                        }

                        line_offset_acc += line_len + 1;
                    }
                }
                _ => { // Paragraph
                    let mut para_y = screen_y + 4;
                    let max_c = ((viewport_w - 40) / 8).max(10) as usize;
                    let font_body = fonts.map(|f| &f.body);

                    for l in block_str.lines() {
                        let clean = l.trim();
                        if clean.starts_with("![") && clean.contains("](") && clean.ends_with(')') {
                            // Relative Image Card
                            let close_b = clean.find(']').unwrap_or(2);
                            let alt_text = &clean[2..close_b];
                            let path_text = &clean[close_b + 2..clean.len() - 1];

                            let img_box = Rect::new(origin_x + 20, para_y, viewport_w - 40, 48);
                            canvas.fill_rect(img_box, 0xFF_21_25_2B);
                            canvas.fill_rect(Rect::new(img_box.x, img_box.y, img_box.width, 1), 0xFF_3E_44_51);
                            canvas.fill_rect(Rect::new(img_box.x, img_box.bottom() - 1, img_box.width, 1), 0xFF_3E_44_51);
                            canvas.fill_rect(Rect::new(img_box.x, img_box.y, 1, img_box.height), 0xFF_3E_44_51);
                            canvas.fill_rect(Rect::new(img_box.right() - 1, img_box.y, 1, img_box.height), 0xFF_3E_44_51);

                            if let Some(f) = fonts {
                                f.ui_bold.draw_text(canvas, "[IMAGE]", img_box.x + 12, img_box.y + 10, 0xFF_61_AF_EF);
                                f.body.draw_text(canvas, alt_text, img_box.x + 72, img_box.y + 10, 0xFF_FF_FF_FF);
                                f.ui.draw_text(canvas, path_text, img_box.x + 12, img_box.y + 28, 0xFF_5C_63_70);
                            } else {
                                canvas.draw_text("[IMAGE]", img_box.x + 12, img_box.y + 10, 0xFF_61_AF_EF);
                                canvas.draw_text(alt_text, img_box.x + 72, img_box.y + 10, 0xFF_FF_FF_FF);
                                canvas.draw_text(path_text, img_box.x + 12, img_box.y + 28, 0xFF_5C_63_70);
                            }

                            para_y += 54;
                        } else {
                            wrap_and_render_inline(canvas, clean, origin_x + 20, &mut para_y, max_c, 0xFF_D4_D4_D4, font_body, &mut click_targets);
                        }
                    }
                }
            }
        }

        click_targets
    }
}

/// Checks if a line is a Markdown table separator like `|---|---|`
#[inline(always)]
fn is_table_separator(line: &str) -> bool {
    let t = line.trim();
    t.starts_with('|') && t.chars().all(|c| c == '|' || c == '-' || c == ':' || c == ' ' || c == '\t')
}

/// Computes how many visual lines a string will occupy when wrapped at `max_chars`.
fn count_wrapped_lines(text: &str, max_chars: usize) -> usize {
    if text.is_empty() {
        return 1;
    }
    let mut count = 0;
    let mut start = 0;
    while start < text.len() {
        let rem = &text[start..];
        if rem.len() <= max_chars {
            count += 1;
            break;
        }
        let candidate = &rem[..max_chars];
        let split = match candidate.rfind(' ') {
            Some(idx) if idx > 0 => idx,
            _ => max_chars,
        };
        count += 1;
        start += split;
        while start < text.len() && text.as_bytes()[start] == b' ' {
            start += 1;
        }
    }
    count.max(1)
}

/// Wraps text to multiple lines and renders each line with inline markdown formatting.
#[cfg(feature = "alloc")]
#[allow(clippy::too_many_arguments)]
fn wrap_and_render_inline(
    canvas: &mut Canvas,
    text: &str,
    x: i32,
    curr_y: &mut i32,
    max_chars: usize,
    default_color: u32,
    font: Option<&CachedFont>,
    click_targets: &mut Vec<ClickableTarget>,
) {
    if text.is_empty() {
        *curr_y += 18;
        return;
    }
    let mut start = 0;
    while start < text.len() {
        let remaining = &text[start..];
        if remaining.len() <= max_chars {
            render_inline_markdown(canvas, remaining, x, *curr_y, default_color, font, click_targets);
            *curr_y += 18;
            break;
        }

        // Find last space before max_chars for clean word wrap
        let candidate = &remaining[..max_chars];
        let split_at = match candidate.rfind(' ') {
            Some(idx) if idx > 0 => idx,
            _ => max_chars,
        };

        render_inline_markdown(canvas, &remaining[..split_at], x, *curr_y, default_color, font, click_targets);
        *curr_y += 18;
        start += split_at;
        while start < text.len() && text.as_bytes()[start] == b' ' {
            start += 1;
        }
    }
}

/// Renders an inline markdown string with formatting (bold, italic, code, strike, links).
#[cfg(feature = "alloc")]
fn render_inline_markdown(
    canvas: &mut Canvas,
    line: &str,
    mut x: i32,
    y: i32,
    default_color: u32,
    font: Option<&CachedFont>,
    click_targets: &mut Vec<ClickableTarget>,
) {
    let bytes = line.as_bytes();
    let mut i = 0;

    while i < bytes.len() {
        // Bold **...**
        if i + 1 < bytes.len() && bytes[i] == b'*' && bytes[i + 1] == b'*' {
            if let Some(end) = line[i + 2..].find("**") {
                let bold_text = &line[i + 2..i + 2 + end];
                if let Some(f) = font {
                    let w = f.draw_text(canvas, bold_text, x, y, 0xFF_FF_FF_FF);
                    f.draw_text(canvas, bold_text, x + 1, y, 0xFF_FF_FF_FF);
                    x += w;
                } else {
                    canvas.draw_text(bold_text, x, y, 0xFF_FF_FF_FF);
                    canvas.draw_text(bold_text, x + 1, y, 0xFF_FF_FF_FF);
                    x += (bold_text.len() as i32) * 8;
                }
                i += 4 + end;
                continue;
            }
        }

        // Strikethrough ~~...~~
        if i + 1 < bytes.len() && bytes[i] == b'~' && bytes[i + 1] == b'~' {
            if let Some(end) = line[i + 2..].find("~~") {
                let strike_text = &line[i + 2..i + 2 + end];
                let w = if let Some(f) = font {
                    f.draw_text(canvas, strike_text, x, y, 0xFF_7F_84_8E)
                } else {
                    canvas.draw_text(strike_text, x, y, 0xFF_7F_84_8E);
                    (strike_text.len() as i32) * 8
                };
                canvas.fill_rect(Rect::new(x, y + 8, w, 1), 0xFF_7F_84_8E);
                x += w;
                i += 4 + end;
                continue;
            }
        }

        // Inline Code `...`
        if bytes[i] == b'`' {
            if let Some(end) = line[i + 1..].find('`') {
                let code_text = &line[i + 1..i + 1 + end];
                let (tw, th) = if let Some(f) = font {
                    f.measure_text(code_text)
                } else {
                    ((code_text.len() as i32) * 8, 16)
                };
                let w = tw + 8;
                canvas.fill_rect(Rect::new(x, y - 1, w, th + 2), 0xFF_28_2C_34);
                if let Some(f) = font {
                    f.draw_text(canvas, code_text, x + 4, y, 0xFF_E0_6C_75);
                } else {
                    canvas.draw_text(code_text, x + 3, y, 0xFF_E0_6C_75);
                }
                x += w + 2;
                i += 2 + end;
                continue;
            }
        }

        // Wikilinks [[target]] or [[target|label]]
        if bytes[i] == b'[' && i + 1 < bytes.len() && bytes[i + 1] == b'[' {
            if let Some(close_bb) = line[i + 2..].find("]]") {
                let inside = &line[i + 2..i + 2 + close_bb];
                let (target, label) = if let Some(pipe_pos) = inside.find('|') {
                    (&inside[..pipe_pos], &inside[pipe_pos + 1..])
                } else {
                    (inside, inside)
                };
                let w = if let Some(f) = font {
                    f.draw_text(canvas, label, x, y, 0xFF_98_C3_79)
                } else {
                    canvas.draw_text(label, x, y, 0xFF_98_C3_79);
                    (label.len() as i32) * 8
                };
                let link_rect = Rect::new(x, y, w, 18);
                canvas.fill_rect(Rect::new(x, y + 16, w, 1), 0xFF_98_C3_79);

                let target_clean = target.trim();
                let doc_url = if target_clean.ends_with(".md") {
                    target_clean.to_string()
                } else {
                    format!("{}.md", target_clean)
                };

                click_targets.push(ClickableTarget::Link {
                    rect: link_rect,
                    url: doc_url,
                });

                x += w;
                i += 2 + close_bb + 2;
                continue;
            }
        }

        // Links [label](url)
        if bytes[i] == b'[' {
            if let Some(close_b) = line[i + 1..].find(']') {
                let rest = &line[i + 1 + close_b + 1..];
                if rest.starts_with('(') {
                    if let Some(close_p) = rest.find(')') {
                        let label = &line[i + 1..i + 1 + close_b];
                        let url = &rest[1..close_p];
                        let w = if let Some(f) = font {
                            f.draw_text(canvas, label, x, y, 0xFF_61_AF_EF)
                        } else {
                            canvas.draw_text(label, x, y, 0xFF_61_AF_EF);
                            (label.len() as i32) * 8
                        };
                        let link_rect = Rect::new(x, y, w, 18);
                        canvas.fill_rect(Rect::new(x, y + 16, w, 1), 0xFF_61_AF_EF);

                        click_targets.push(ClickableTarget::Link {
                            rect: link_rect,
                            url: url.to_string(),
                        });

                        x += w;
                        i += 1 + close_b + 1 + close_p + 1;
                        continue;
                    }
                }
            }
        }

        // Italic *...*
        if bytes[i] == b'*' {
            if let Some(end) = line[i + 1..].find('*') {
                let italic_text = &line[i + 1..i + 1 + end];
                let w = if let Some(f) = font {
                    f.draw_text(canvas, italic_text, x, y, 0xFF_E5_C0_7B)
                } else {
                    canvas.draw_text(italic_text, x, y, 0xFF_E5_C0_7B);
                    (italic_text.len() as i32) * 8
                };
                x += w;
                i += 2 + end;
                continue;
            }
        }

        let ch = line[i..].chars().next().unwrap_or(' ');
        if let Some(f) = font {
            let mut buf = [0u8; 4];
            let s = ch.encode_utf8(&mut buf);
            x += f.draw_text(canvas, s, x, y, default_color);
        } else {
            canvas.draw_char(ch, x, y, default_color);
            x += 8;
        }
        i += ch.len_utf8();
    }
}
