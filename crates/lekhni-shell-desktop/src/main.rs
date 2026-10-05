//! Lekhni Desktop Interactive Native GUI Runner.
//!
//! Pure Rust native desktop frontend. No Webview, no Electron, no Tauri.
//! Direct zero-copy presentation to Linux display surface with zero idle CPU.

use std::sync::mpsc::{channel, Receiver};
use std::thread;
use std::io::Read;
use std::time::{Duration, Instant};

use lekhni_layout::{compute_workspace_layout_with_mode, WorkspaceMode};
use lekhni_md::parser::MarkdownParser;
use lekhni_raster::canvas::Canvas;
use lekhni_raster::damage::Rect;
use lekhni_shell_desktop::workspace::{get_or_init_working_dir, NotebookManager, StdFs};
use lekhni_shell_desktop::x11_presenter::{KeyAction, X11Window};
use lekhni_text::buffer::PieceTable;
use lekhni_text::cursor::{Cursor, Selection};
use lekhni_text::undo::UndoManager;
use lekhni_ui::preview::{ClickableTarget, PreviewRenderer};
use lekhni_ui::selection::SelectionHandler;
use lekhni_ui::theme::Theme;
use lekhni_store::recovery::RecoveryJournal;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum ActiveFocus {
    Sidebar,
    PageList,
    Editor,
}

/// Linux desktop clipboard supporting system clipboard (xclip/wl-clipboard) and memory fallback.
struct DesktopClipboard {
    internal: String,
}

impl DesktopClipboard {
    fn new() -> Self {
        Self { internal: String::new() }
    }
}

impl lekhni_shell::Clipboard for DesktopClipboard {
    fn get_text(&self) -> Option<String> {
        if let Ok(output) = std::process::Command::new("xclip").args(["-selection", "clipboard", "-o"]).output() {
            if output.status.success() {
                if let Ok(s) = String::from_utf8(output.stdout) {
                    if !s.is_empty() {
                        return Some(s);
                    }
                }
            }
        }
        if let Ok(output) = std::process::Command::new("wl-paste").output() {
            if output.status.success() {
                if let Ok(s) = String::from_utf8(output.stdout) {
                    if !s.is_empty() {
                        return Some(s);
                    }
                }
            }
        }
        if !self.internal.is_empty() {
            Some(self.internal.clone())
        } else {
            None
        }
    }

    fn set_text(&mut self, text: &str) {
        self.internal = text.to_string();
        if let Ok(mut child) = std::process::Command::new("xclip")
            .args(["-selection", "clipboard", "-i"])
            .stdin(std::process::Stdio::piped())
            .spawn()
        {
            if let Some(mut stdin) = child.stdin.take() {
                use std::io::Write;
                let _ = stdin.write_all(text.as_bytes());
            }
            let _ = child.wait();
        }
        if let Ok(mut child) = std::process::Command::new("wl-copy")
            .stdin(std::process::Stdio::piped())
            .spawn()
        {
            if let Some(mut stdin) = child.stdin.take() {
                use std::io::Write;
                let _ = stdin.write_all(text.as_bytes());
            }
            let _ = child.wait();
        }
    }
}

fn prev_char_boundary(buffer: &PieceTable, mut offset: usize) -> usize {
    if offset == 0 || buffer.is_empty() {
        return 0;
    }
    offset = offset.min(buffer.len());
    offset -= 1;
    while offset > 0 {
        if let Some(b) = buffer.byte_at(offset) {
            if (b & 0xC0) != 0x80 {
                break;
            }
        }
        offset -= 1;
    }
    offset
}

fn next_char_boundary(buffer: &PieceTable, mut offset: usize) -> usize {
    let len = buffer.len();
    if offset >= len {
        return len;
    }
    offset += 1;
    while offset < len {
        if let Some(b) = buffer.byte_at(offset) {
            if (b & 0xC0) != 0x80 {
                break;
            }
        }
        offset += 1;
    }
    offset.min(len)
}

fn offset_from_line_col(buffer: &PieceTable, target_line: usize, target_col: usize) -> usize {
    let line_start = buffer.line_to_offset(target_line);
    if line_start >= buffer.len() {
        return buffer.len();
    }
    let mut curr = line_start;
    let mut col = 0;
    while curr < buffer.len() && col < target_col {
        if let Some(b) = buffer.byte_at(curr) {
            if b == b'\n' {
                break;
            }
        }
        curr = next_char_boundary(buffer, curr);
        col += 1;
    }
    curr
}

fn find_line_end(buffer: &PieceTable, offset: usize) -> usize {
    let mut curr = offset;
    while curr < buffer.len() {
        if let Some(b) = buffer.byte_at(curr) {
            if b == b'\n' {
                break;
            }
        }
        curr += 1;
    }
    curr
}

/// Renders a single line of Markdown source with syntax highlighting in the editor pane.
fn render_syntax_highlighted_editor_line(canvas: &mut Canvas, line: &str, mut x: i32, y: i32) {
    let trimmed = line.trim_start();
    if trimmed.starts_with("# ") {
        let prefix_len = line.len() - trimmed.len() + 2;
        canvas.draw_text(&line[..prefix_len], x, y, 0xFF_E0_6C_75);
        x += (prefix_len as i32) * 8;
        canvas.draw_text(&line[prefix_len..], x, y, 0xFF_61_AF_EF);
        canvas.draw_text(&line[prefix_len..], x + 1, y, 0xFF_61_AF_EF); // bold
        return;
    } else if trimmed.starts_with("## ") {
        let prefix_len = line.len() - trimmed.len() + 3;
        canvas.draw_text(&line[..prefix_len], x, y, 0xFF_E0_6C_75);
        x += (prefix_len as i32) * 8;
        canvas.draw_text(&line[prefix_len..], x, y, 0xFF_61_AF_EF);
        return;
    } else if trimmed.starts_with("### ") {
        let prefix_len = line.len() - trimmed.len() + 4;
        canvas.draw_text(&line[..prefix_len], x, y, 0xFF_E0_6C_75);
        x += (prefix_len as i32) * 8;
        canvas.draw_text(&line[prefix_len..], x, y, 0xFF_4E_C9_B0);
        return;
    } else if trimmed.starts_with("> ") {
        let prefix_len = line.len() - trimmed.len() + 2;
        canvas.draw_text(&line[..prefix_len], x, y, 0xFF_00_7A_CC);
        x += (prefix_len as i32) * 8;
        canvas.draw_text(&line[prefix_len..], x, y, 0xFF_9C_DC_FE);
        return;
    } else if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
        canvas.draw_text(line, x, y, 0xFF_E5_C0_7B);
        return;
    } else if trimmed.starts_with("- [ ] ") {
        let prefix_len = line.len() - trimmed.len() + 6;
        canvas.draw_text(&line[..prefix_len], x, y, 0xFF_E5_C0_7B);
        x += (prefix_len as i32) * 8;
        canvas.draw_text(&line[prefix_len..], x, y, 0xFF_AB_B2_BF);
        return;
    } else if trimmed.starts_with("- [x] ") {
        let prefix_len = line.len() - trimmed.len() + 6;
        canvas.draw_text(&line[..prefix_len], x, y, 0xFF_98_C3_79);
        x += (prefix_len as i32) * 8;
        canvas.draw_text(&line[prefix_len..], x, y, 0xFF_98_C3_79);
        return;
    } else if trimmed.starts_with("- ") || trimmed.starts_with("* ") || trimmed.starts_with("+ ") {
        let prefix_len = line.len() - trimmed.len() + 2;
        canvas.draw_text(&line[..prefix_len], x, y, 0xFF_61_AF_EF);
        x += (prefix_len as i32) * 8;
        canvas.draw_text(&line[prefix_len..], x, y, 0xFF_AB_B2_BF);
        return;
    } else if trimmed.starts_with('|') && trimmed.ends_with('|') {
        // Table row syntax highlighting
        for ch in line.chars() {
            let color = if ch == '|' { 0xFF_5C_63_70 } else { 0xFF_AB_B2_BF };
            canvas.draw_char(ch, x, y, color);
            x += 8;
        }
        return;
    }

    // Regular line inline spans syntax highlighting
    let bytes = line.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if i + 1 < bytes.len() && bytes[i] == b'*' && bytes[i + 1] == b'*' {
            if let Some(end) = line[i + 2..].find("**") {
                let bold_full = &line[i..i + 4 + end];
                canvas.draw_text(bold_full, x, y, 0xFF_FF_FF_FF);
                canvas.draw_text(bold_full, x + 1, y, 0xFF_FF_FF_FF);
                x += (bold_full.len() as i32) * 8;
                i += 4 + end;
                continue;
            }
        }
        if i + 1 < bytes.len() && bytes[i] == b'~' && bytes[i + 1] == b'~' {
            if let Some(end) = line[i + 2..].find("~~") {
                let strike_full = &line[i..i + 4 + end];
                canvas.draw_text(strike_full, x, y, 0xFF_7F_84_8E);
                x += (strike_full.len() as i32) * 8;
                i += 4 + end;
                continue;
            }
        }
        if bytes[i] == b'`' {
            if let Some(end) = line[i + 1..].find('`') {
                let code_full = &line[i..i + 2 + end];
                canvas.draw_text(code_full, x, y, 0xFF_E0_6C_75);
                x += (code_full.len() as i32) * 8;
                i += 2 + end;
                continue;
            }
        }
        if bytes[i] == b'[' {
            if let Some(close_b) = line[i + 1..].find(']') {
                let rest = &line[i + 1 + close_b + 1..];
                if rest.starts_with('(') {
                    if let Some(close_p) = rest.find(')') {
                        let total_len = 1 + close_b + 1 + 1 + close_p + 1;
                        let link_full = &line[i..i + total_len];
                        canvas.draw_text(link_full, x, y, 0xFF_61_AF_EF);
                        x += (link_full.len() as i32) * 8;
                        i += total_len;
                        continue;
                    }
                }
            }
        }

        let ch = line[i..].chars().next().unwrap_or(' ');
        canvas.draw_char(ch, x, y, 0xFF_AB_B2_BF);
        x += 8;
        i += ch.len_utf8();
    }
}

fn main() {
    println!("=== Launching Lekhni Pure Rust Native Editor ===");

    // 1. Initialize or select working directory on first launch
    let working_dir = get_or_init_working_dir();
    println!("Working directory: {}", working_dir.display());

    let mut nb_mgr = NotebookManager::new(working_dir.clone());
    println!(
        "Active notebook: '{}' ({} total), Active page: '{}' ({} total)",
        nb_mgr.notebooks.get(nb_mgr.active_notebook_idx).map(|s| s.as_str()).unwrap_or("None"),
        nb_mgr.notebooks.len(),
        nb_mgr.pages.get(nb_mgr.active_page_idx).map(|s| s.as_str()).unwrap_or("None"),
        nb_mgr.pages.len()
    );

    let mut width = 1100u16;
    let mut height = 700u16;
    let mut sidebar_w = 190i32;
    let mut page_list_w = 210i32;
    let mut dragging_splitter: Option<usize> = None;
    let mut dragging_selection = false;
    let mut view_mode = WorkspaceMode::Split;

    let theme = Theme::dark();
    let mut fb = vec![theme.bg_workspace; (width as usize) * (height as usize)];

    // 2. Load active document into PieceTable and parser
    let mut doc_memory = nb_mgr.load_active_content();
    let mut buffer = PieceTable::new(&doc_memory);
    let mut cursor_pos: usize = buffer.len();
    let mut selection = Selection::collapsed(Cursor::new(cursor_pos, 0, 0));
    let mut undo_mgr = UndoManager::new();
    let mut clipboard = DesktopClipboard::new();
    let mut parser = MarkdownParser::new();
    let mut focus = ActiveFocus::Editor;
    let mut editor_scroll_y: i32 = 0;
    let mut scroll_y: i32 = 0;
    let mut is_dirty = false;
    let mut last_edit_time = Instant::now();
    let mut status_msg = String::from("Ready. Phase 2 live: rich markdown, interactive links/tasks, split lock.");

    // Check for existing crash recovery journal on startup
    if let (Some(nb_path), Some(page_name)) = (nb_mgr.active_notebook_dir(), nb_mgr.pages.get(nb_mgr.active_page_idx)) {
        let rec_path = RecoveryJournal::journal_path(&nb_path.display().to_string(), page_name);
        if let Ok(rec_bytes) = std::fs::read(&rec_path) {
            if !rec_bytes.is_empty() && rec_bytes != doc_memory {
                println!("Found recovery journal snapshot from previous session for '{}'", page_name);
                doc_memory = rec_bytes;
                buffer = PieceTable::new(&doc_memory);
                cursor_pos = buffer.len();
                selection = Selection::collapsed(Cursor::new(cursor_pos, 0, 0));
                is_dirty = true;
                status_msg = "Restored uncommitted session from recovery journal!".into();
            }
        }
    }

    // 3. Open native X11 window
    println!("Connecting to display and opening native window...");
    let mut win = match X11Window::open("Lekhni Markdown Editor (Native Pure Rust)", width, height) {
        Ok(w) => w,
        Err(e) => {
            eprintln!("Native display presentation notice: {}", e);
            eprintln!("Desktop headless and benchmark modes remain fully functional.");
            return;
        }
    };

    println!("Window mapped successfully! Window ID: 0x{:x}", win.window_id);

    // 4. Redraw pass
    let redraw = |fb: &mut [u32],
                      win_w: u16,
                      win_h: u16,
                      sb_w: i32,
                      pl_w: i32,
                      active_drag: Option<usize>,
                      buffer: &PieceTable,
                      parser: &mut MarkdownParser,
                      nb_mgr: &NotebookManager,
                      focus: ActiveFocus,
                      cursor_pos: usize,
                      selection: &Selection,
                      editor_scroll_y: i32,
                      scroll_y: i32,
                      dirty: bool,
                      mode: WorkspaceMode,
                      status: &str| -> Vec<ClickableTarget> {
        let mut canvas = Canvas::new(fb, win_w as u32, win_h as u32);
        canvas.clear(theme.bg_workspace);

        let menu_h = 28;
        let status_h = 24;
        let layout = compute_workspace_layout_with_mode(
            win_w as i32,
            win_h as i32,
            sb_w,
            pl_w,
            3,
            mode,
            menu_h,
            status_h,
        );

        // A. Top Menu Bar
        canvas.fill_rect(layout.menu_bar_rect, 0xFF_21_25_2B);
        canvas.draw_text("LEKHNI", 12, 6, 0xFF_61_AF_EF);

        // Interactive Pane Focus Pills in Header
        let pill_sidebar = Rect::new(80, 4, 100, 20);
        let pill_pages = Rect::new(186, 4, 85, 20);
        let pill_editor = Rect::new(277, 4, 90, 20);

        canvas.fill_rect(pill_sidebar, if focus == ActiveFocus::Sidebar { 0xFF_00_7A_CC } else { 0xFF_2D_31_39 });
        canvas.draw_text("[1] NOTES", 86, 6, if focus == ActiveFocus::Sidebar { 0xFF_FF_FF_FF } else { 0xFF_AB_B2_BF });

        canvas.fill_rect(pill_pages, if focus == ActiveFocus::PageList { 0xFF_00_7A_CC } else { 0xFF_2D_31_39 });
        canvas.draw_text("[2] PAGES", 192, 6, if focus == ActiveFocus::PageList { 0xFF_FF_FF_FF } else { 0xFF_AB_B2_BF });

        canvas.fill_rect(pill_editor, if focus == ActiveFocus::Editor { 0xFF_00_7A_CC } else { 0xFF_2D_31_39 });
        canvas.draw_text("[3] EDIT", 283, 6, if focus == ActiveFocus::Editor { 0xFF_FF_FF_FF } else { 0xFF_AB_B2_BF });

        // View Mode Selector Pills (F1: Split, F2: Editor, F3: Preview)
        let pill_m_split = Rect::new(380, 4, 75, 20);
        let pill_m_edit = Rect::new(460, 4, 75, 20);
        let pill_m_prev = Rect::new(540, 4, 80, 20);

        canvas.fill_rect(pill_m_split, if mode == WorkspaceMode::Split { 0xFF_00_7A_CC } else { 0xFF_2D_31_39 });
        canvas.draw_text("[F1] SPLIT", 384, 6, if mode == WorkspaceMode::Split { 0xFF_FF_FF_FF } else { 0xFF_AB_B2_BF });

        canvas.fill_rect(pill_m_edit, if mode == WorkspaceMode::EditorOnly { 0xFF_00_7A_CC } else { 0xFF_2D_31_39 });
        canvas.draw_text("[F2] EDIT", 466, 6, if mode == WorkspaceMode::EditorOnly { 0xFF_FF_FF_FF } else { 0xFF_AB_B2_BF });

        canvas.fill_rect(pill_m_prev, if mode == WorkspaceMode::PreviewOnly { 0xFF_00_7A_CC } else { 0xFF_2D_31_39 });
        canvas.draw_text("[F3] PREV", 546, 6, if mode == WorkspaceMode::PreviewOnly { 0xFF_FF_FF_FF } else { 0xFF_AB_B2_BF });

        let active_nb_name = nb_mgr.notebooks.get(nb_mgr.active_notebook_idx).map(|s| s.as_str()).unwrap_or("None");
        let active_pg_name = nb_mgr.pages.get(nb_mgr.active_page_idx).map(|s| s.as_str()).unwrap_or("None");
        let right_label = format!("{}/{}", active_nb_name, active_pg_name);
        let right_x = (win_w as i32) - (right_label.len() as i32 * 8) - 16;
        if right_x > 630 {
            canvas.draw_text(&right_label, right_x, 6, 0xFF_E5_C0_7B);
        }

        // B. Sidebar (Notebooks)
        let sb_bg = if focus == ActiveFocus::Sidebar { 0xFF_1E_22_27 } else { theme.bg_sidebar };
        canvas.fill_rect(layout.sidebar_rect, sb_bg);

        if focus == ActiveFocus::Sidebar {
            canvas.fill_rect(Rect::new(layout.sidebar_rect.x, layout.sidebar_rect.y, layout.sidebar_rect.width, 3), 0xFF_00_7A_CC);
            canvas.fill_rect(Rect::new(layout.sidebar_rect.x + 4, layout.sidebar_rect.y + 6, layout.sidebar_rect.width - 8, 20), 0xFF_2D_31_39);
            canvas.draw_text("=== [ NOTEBOOKS ] ===", layout.sidebar_rect.x + 12, layout.sidebar_rect.y + 8, 0xFF_61_AF_EF);
        } else {
            canvas.draw_text("=== NOTEBOOKS ===", layout.sidebar_rect.x + 12, layout.sidebar_rect.y + 8, 0xFF_5C_63_70);
        }

        let mut nb_y = layout.sidebar_rect.y + 34;
        for (i, nb) in nb_mgr.notebooks.iter().enumerate() {
            if nb_y + 20 > layout.sidebar_rect.bottom() {
                break;
            }
            let is_active = i == nb_mgr.active_notebook_idx;
            if is_active {
                canvas.fill_rect(
                    Rect::new(layout.sidebar_rect.x + 4, nb_y - 2, layout.sidebar_rect.width - 8, 22),
                    if focus == ActiveFocus::Sidebar { 0xFF_09_47_71 } else { 0xFF_2C_31_3C },
                );
                canvas.fill_rect(Rect::new(layout.sidebar_rect.x + 4, nb_y - 2, 3, 22), 0xFF_00_7A_CC);
                canvas.draw_text(&format!("* {}", nb), layout.sidebar_rect.x + 10, nb_y, 0xFF_FF_FF_FF);
            } else {
                canvas.draw_text(&format!("  {}", nb), layout.sidebar_rect.x + 10, nb_y, 0xFF_AB_B2_BF);
            }
            nb_y += 24;
        }

        // C. Splitter 1 (Draggable)
        let s1_color = if active_drag == Some(1) { 0xFF_00_7A_CC } else { theme.border_color };
        canvas.fill_rect(layout.splitter_1_rect, s1_color);

        // D. Page List (Notes in Active Notebook)
        let pl_bg = if focus == ActiveFocus::PageList { 0xFF_21_25_2B } else { theme.bg_page_list };
        canvas.fill_rect(layout.page_list_rect, pl_bg);

        if focus == ActiveFocus::PageList {
            canvas.fill_rect(Rect::new(layout.page_list_rect.x, layout.page_list_rect.y, layout.page_list_rect.width, 3), 0xFF_00_7A_CC);
            canvas.fill_rect(Rect::new(layout.page_list_rect.x + 4, layout.page_list_rect.y + 6, layout.page_list_rect.width - 8, 20), 0xFF_2D_31_39);
            canvas.draw_text("=== [ PAGES ] ===", layout.page_list_rect.x + 16, layout.page_list_rect.y + 8, 0xFF_61_AF_EF);
        } else {
            canvas.draw_text("=== PAGES ===", layout.page_list_rect.x + 16, layout.page_list_rect.y + 8, 0xFF_5C_63_70);
        }

        let mut pg_y = layout.page_list_rect.y + 34;
        for (i, page) in nb_mgr.pages.iter().enumerate() {
            if pg_y + 20 > layout.page_list_rect.bottom() {
                break;
            }
            let is_active = i == nb_mgr.active_page_idx;
            if is_active {
                canvas.fill_rect(
                    Rect::new(layout.page_list_rect.x + 4, pg_y - 2, layout.page_list_rect.width - 8, 22),
                    if focus == ActiveFocus::PageList { 0xFF_09_47_71 } else { 0xFF_2C_31_3C },
                );
                canvas.fill_rect(Rect::new(layout.page_list_rect.x + 4, pg_y - 2, 3, 22), 0xFF_00_7A_CC);
                canvas.draw_text(&format!("* {}", page), layout.page_list_rect.x + 10, pg_y, 0xFF_FF_FF_FF);
            } else {
                canvas.draw_text(&format!("  {}", page), layout.page_list_rect.x + 10, pg_y, 0xFF_AB_B2_BF);
            }
            pg_y += 24;
        }

        // E. Splitter 2 (Draggable)
        let s2_color = if active_drag == Some(2) { 0xFF_00_7A_CC } else { theme.border_color };
        canvas.fill_rect(layout.splitter_2_rect, s2_color);

        let mut doc_bytes = vec![0u8; buffer.len()];
        buffer.copy_range(0, buffer.len(), &mut doc_bytes);
        let doc_str = String::from_utf8_lossy(&doc_bytes);

        // F. Editor Pane (Rendered if mode is Split or EditorOnly)
        if mode != WorkspaceMode::PreviewOnly && layout.editor_rect.width > 0 {
            let ed_bg = if focus == ActiveFocus::Editor { 0xFF_28_2C_34 } else { 0xFF_21_25_2B };
            canvas.fill_rect(layout.editor_rect, ed_bg);
            if focus == ActiveFocus::Editor {
                canvas.fill_rect(Rect::new(layout.editor_rect.x, layout.editor_rect.y, layout.editor_rect.width, 3), 0xFF_00_7A_CC);
            }

            // Gutter
            let gutter_w = 44;
            canvas.fill_rect(
                Rect::new(layout.editor_rect.x, layout.editor_rect.y, gutter_w, layout.editor_rect.height),
                0xFF_21_25_2B,
            );
            canvas.fill_rect(Rect::new(layout.editor_rect.x + gutter_w - 1, layout.editor_rect.y, 1, layout.editor_rect.height), 0xFF_3E_44_51);

            // Clip canvas strictly to Editor viewport
            canvas.clip = layout.editor_rect;

            let mut line_num = 1;
            let mut ed_y = layout.editor_rect.y + 10 - editor_scroll_y;
            let mut char_count_acc = 0usize;
            let (sel_start, sel_end) = selection.byte_range();

            for line in doc_str.split('\n') {
                let line_len = line.len();
                let line_start = char_count_acc;
                let line_end = line_start + line_len;

                if ed_y + 18 >= layout.editor_rect.y && ed_y <= layout.editor_rect.bottom() {
                    // Line number in gutter
                    let num_str = format!("{:3}", line_num);
                    canvas.draw_text(&num_str, layout.editor_rect.x + 6, ed_y, 0xFF_5C_63_70);

                    let text_x = layout.editor_rect.x + gutter_w + 10;

                    // Selection highlight behind text
                    if !selection.is_collapsed() && sel_start < line_end && sel_end > line_start {
                        let hl_start = sel_start.max(line_start) - line_start;
                        let hl_end = sel_end.min(line_end) - line_start;
                        let hl_x1 = text_x + (hl_start as i32) * 8;
                        let hl_x2 = text_x + (hl_end as i32) * 8;
                        canvas.fill_rect(Rect::new(hl_x1, ed_y, (hl_x2 - hl_x1).max(4), 18), 0xFF_26_4F_78);
                    }

                    // Draw line text with Markdown syntax highlighting
                    render_syntax_highlighted_editor_line(&mut canvas, line, text_x, ed_y);

                    // Caret cursor
                    if focus == ActiveFocus::Editor && cursor_pos >= line_start && cursor_pos <= line_end {
                        let col = (cursor_pos - line_start) as i32;
                        let cur_x = text_x + col * 8;
                        if cur_x + 2 < layout.editor_rect.right() {
                            canvas.fill_rect(Rect::new(cur_x, ed_y, 2, 16), 0xFF_52_8B_FF);
                        }
                    }
                }

                char_count_acc += line_len + 1; // + 1 for \n
                line_num += 1;
                ed_y += 18;
            }

            // If buffer ends with newline or is empty, draw cursor on final line
            if char_count_acc <= cursor_pos && ed_y + 18 >= layout.editor_rect.y && ed_y <= layout.editor_rect.bottom() {
                let num_str = format!("{:3}", line_num);
                canvas.draw_text(&num_str, layout.editor_rect.x + 6, ed_y, 0xFF_5C_63_70);
                if focus == ActiveFocus::Editor {
                    let text_x = layout.editor_rect.x + gutter_w + 10;
                    canvas.fill_rect(Rect::new(text_x, ed_y, 2, 16), 0xFF_52_8B_FF);
                }
            }

            // Restore global clip
            canvas.clip = Rect::new(0, 0, win_w as i32, win_h as i32);
        }

        // Splitter 3 (Between Editor and Preview in Split Mode)
        if mode == WorkspaceMode::Split {
            let s3_color = if active_drag == Some(3) { 0xFF_00_7A_CC } else { theme.border_color };
            canvas.fill_rect(layout.splitter_3_rect, s3_color);
        }

        // G. Preview Pane (Rendered if mode is Split or PreviewOnly)
        let mut click_targets = Vec::new();
        if mode != WorkspaceMode::EditorOnly && layout.preview_rect.width > 0 {
            canvas.fill_rect(layout.preview_rect, 0xFF_1E_22_27);
            parser.parse_full(&doc_bytes);

            canvas.clip = layout.preview_rect;
            click_targets = PreviewRenderer::render_preview_with_content(
                &mut canvas,
                &parser.blocks,
                &doc_bytes,
                scroll_y,
                layout.preview_rect.width,
                layout.preview_rect.height,
            );
            canvas.clip = Rect::new(0, 0, win_w as i32, win_h as i32);
        }

        // H. Status Bar
        let sb_bar_bg = if dirty { 0xFF_D1_9A_66 } else { 0xFF_00_7A_CC };
        canvas.fill_rect(layout.status_bar_rect, sb_bar_bg);
        let word_count = doc_str.split_whitespace().count();
        let (caret_line, line_start) = buffer.offset_to_line(cursor_pos);
        let caret_col = cursor_pos.saturating_sub(line_start);
        let dirty_flag = if dirty { " [DIRTY]" } else { "" };
        let mode_str = match mode {
            WorkspaceMode::Split => "Split",
            WorkspaceMode::EditorOnly => "Editor",
            WorkspaceMode::PreviewOnly => "Preview",
        };
        let status_text = format!(
            " {}{} | Mode: {} | Ln {}, Col {} | Focus: {:?} | {} bytes, {} words",
            status, dirty_flag, mode_str, caret_line + 1, caret_col + 1, focus, buffer.len(), word_count
        );
        canvas.draw_text(&status_text, 8, layout.status_bar_rect.y + 4, 0xFF_FF_FF_FF);

        click_targets
    };

    let mut current_click_targets = redraw(
        &mut fb, width, height, sidebar_w, page_list_w, dragging_splitter,
        &buffer, &mut parser, &nb_mgr, focus, cursor_pos, &selection,
        editor_scroll_y, scroll_y, is_dirty, view_mode, &status_msg,
    );
    let _ = win.present_framebuffer(&fb, width, height);

    // 5. Stdin bridge for terminal automation
    let (tx_stdin, rx_stdin): (std::sync::mpsc::Sender<u8>, Receiver<u8>) = channel();
    thread::spawn(move || {
        let mut buf = [0u8; 1];
        let mut stdin = std::io::stdin();
        while let Ok(n) = stdin.read(&mut buf) {
            if n == 0 {
                break;
            }
            if tx_stdin.send(buf[0]).is_err() {
                break;
            }
        }
    });

    // 6. Interactive Event Loop with short timeout for autosave
    loop {
        // Handle optional stdin input from piped terminal
        if let Ok(ch) = rx_stdin.try_recv() {
            if ch == 27 {
                println!("\nEscape pressed in terminal. Exiting Lekhni.");
                break;
            } else if ch == 8 || ch == 127 {
                if cursor_pos > 0 && !buffer.is_empty() {
                    undo_mgr.push_undo(buffer.take_snapshot(cursor_pos));
                    let prev = prev_char_boundary(&buffer, cursor_pos);
                    buffer.delete(prev, cursor_pos - prev);
                    cursor_pos = prev;
                    selection = Selection::collapsed(Cursor::new(cursor_pos, 0, 0));
                    is_dirty = true;
                    last_edit_time = Instant::now();
                    status_msg = "Edited".into();
                }
            } else if ch == b'\n' || ch == b'\r' {
                undo_mgr.push_undo(buffer.take_snapshot(cursor_pos));
                buffer.insert(cursor_pos, b"\n");
                cursor_pos += 1;
                selection = Selection::collapsed(Cursor::new(cursor_pos, 0, 0));
                is_dirty = true;
                last_edit_time = Instant::now();
                status_msg = "Edited".into();
            } else if (32..127).contains(&ch) {
                undo_mgr.push_undo(buffer.take_snapshot(cursor_pos));
                buffer.insert(cursor_pos, &[ch]);
                cursor_pos += 1;
                selection = Selection::collapsed(Cursor::new(cursor_pos, 0, 0));
                is_dirty = true;
                last_edit_time = Instant::now();
                status_msg = "Edited".into();
            }
            current_click_targets = redraw(
                &mut fb, width, height, sidebar_w, page_list_w, dragging_splitter,
                &buffer, &mut parser, &nb_mgr, focus, cursor_pos, &selection,
                editor_scroll_y, scroll_y, is_dirty, view_mode, &status_msg,
            );
            let _ = win.present_framebuffer(&fb, width, height);
        }

        // Autosave check: if dirty and 1.5s passed without further edits, flush to disk
        if is_dirty && last_edit_time.elapsed() >= Duration::from_millis(1500) {
            let mut doc_bytes = vec![0u8; buffer.len()];
            buffer.copy_range(0, buffer.len(), &mut doc_bytes);
            if let Ok(()) = nb_mgr.save_active_content(&doc_bytes) {
                if let (Some(nb_path), Some(page_name)) = (nb_mgr.active_notebook_dir(), nb_mgr.pages.get(nb_mgr.active_page_idx)) {
                    let _ = RecoveryJournal::discard_snapshot(&StdFs, &nb_path.display().to_string(), page_name);
                }
                is_dirty = false;
                status_msg = "Autosaved note atomically".into();
                current_click_targets = redraw(
                    &mut fb, width, height, sidebar_w, page_list_w, dragging_splitter,
                    &buffer, &mut parser, &nb_mgr, focus, cursor_pos, &selection,
                    editor_scroll_y, scroll_y, is_dirty, view_mode, &status_msg,
                );
                let _ = win.present_framebuffer(&fb, width, height);
            }
        }

        // Wait for next X11 event with a 50ms timeout for smooth autosave triggers
        match win.wait_event_timeout(Duration::from_millis(50)) {
            Ok(Some(event)) => {
                let event_type = event[0] & 0x7F;
                match event_type {
                    // Expose event (opcode 12): re-present window framebuffer
                    12 => {
                        let _ = win.present_framebuffer(&fb, width, height);
                    }
                    // ConfigureNotify event (opcode 22): window resized or maximized!
                    22 => {
                        let new_w = u16::from_le_bytes([event[20], event[21]]);
                        let new_h = u16::from_le_bytes([event[22], event[23]]);
                        if (new_w != width || new_h != height) && new_w >= 400 && new_h >= 300 {
                            width = new_w;
                            height = new_h;
                            win.width = new_w;
                            win.height = new_h;
                            fb.resize((width as usize) * (height as usize), theme.bg_workspace);
                            status_msg = format!("Resized to {}x{}", width, height);
                            current_click_targets = redraw(
                                &mut fb, width, height, sidebar_w, page_list_w, dragging_splitter,
                                &buffer, &mut parser, &nb_mgr, focus, cursor_pos, &selection,
                                editor_scroll_y, scroll_y, is_dirty, view_mode, &status_msg,
                            );
                            let _ = win.present_framebuffer(&fb, width, height);
                        }
                    }
                    // MotionNotify event (opcode 6): Splitter & selection drag tracking!
                    6 => {
                        let mouse_x = i16::from_le_bytes([event[24], event[25]]) as i32;
                        let mouse_y = i16::from_le_bytes([event[26], event[27]]) as i32;

                        if let Some(dragged) = dragging_splitter {
                            if dragged == 1 {
                                sidebar_w = mouse_x.clamp(100, (width as i32) / 3);
                            } else if dragged == 2 {
                                page_list_w = (mouse_x - sidebar_w).clamp(100, (width as i32) / 3);
                            }
                            current_click_targets = redraw(
                                &mut fb, width, height, sidebar_w, page_list_w, dragging_splitter,
                                &buffer, &mut parser, &nb_mgr, focus, cursor_pos, &selection,
                                editor_scroll_y, scroll_y, is_dirty, view_mode, &status_msg,
                            );
                            let _ = win.present_framebuffer(&fb, width, height);
                        } else if dragging_selection {
                            let layout = compute_workspace_layout_with_mode(
                                width as i32, height as i32, sidebar_w, page_list_w, 3, view_mode, 28, 24,
                            );
                            let gutter_w = 44;
                            let text_x = layout.editor_rect.x + gutter_w + 10;
                            let text_y = layout.editor_rect.y + 10;
                            let target_line = ((mouse_y - text_y + editor_scroll_y) / 18).max(0) as usize;
                            let target_col = ((mouse_x - text_x) / 8).max(0) as usize;
                            let new_off = offset_from_line_col(&buffer, target_line, target_col);
                            selection.head = Cursor::new(new_off, target_line, target_col);
                            cursor_pos = new_off;
                            current_click_targets = redraw(
                                &mut fb, width, height, sidebar_w, page_list_w, dragging_splitter,
                                &buffer, &mut parser, &nb_mgr, focus, cursor_pos, &selection,
                                editor_scroll_y, scroll_y, is_dirty, view_mode, &status_msg,
                            );
                            let _ = win.present_framebuffer(&fb, width, height);
                        }
                    }
                    // ButtonRelease event (opcode 5)
                    5 => {
                        if dragging_splitter.is_some() || dragging_selection {
                            dragging_splitter = None;
                            dragging_selection = false;
                            current_click_targets = redraw(
                                &mut fb, width, height, sidebar_w, page_list_w, dragging_splitter,
                                &buffer, &mut parser, &nb_mgr, focus, cursor_pos, &selection,
                                editor_scroll_y, scroll_y, is_dirty, view_mode, &status_msg,
                            );
                            let _ = win.present_framebuffer(&fb, width, height);
                        }
                    }
                    // ButtonPress event (opcode 4): Mouse clicks and scroll wheel
                    4 => {
                        let button = event[1];
                        let mouse_x = i16::from_le_bytes([event[24], event[25]]) as i32;
                        let mouse_y = i16::from_le_bytes([event[26], event[27]]) as i32;

                        let layout = compute_workspace_layout_with_mode(
                            width as i32, height as i32, sidebar_w, page_list_w, 3, view_mode, 28, 24,
                        );

                        if button == 1 {
                            // Hit-test splitters for dragging
                            if (mouse_x - layout.splitter_1_rect.x).abs() <= 5 {
                                dragging_splitter = Some(1);
                                status_msg = "Dragging Sidebar Splitter".into();
                            } else if (mouse_x - layout.splitter_2_rect.x).abs() <= 5 {
                                dragging_splitter = Some(2);
                                status_msg = "Dragging Page List Splitter".into();
                            } else if mouse_y < 28 {
                                // Click in top header focus or mode pills
                                if (80..180).contains(&mouse_x) {
                                    focus = ActiveFocus::Sidebar;
                                    status_msg = "Focused Notebooks".into();
                                } else if (186..271).contains(&mouse_x) {
                                    focus = ActiveFocus::PageList;
                                    status_msg = "Focused Pages".into();
                                } else if (277..367).contains(&mouse_x) {
                                    focus = ActiveFocus::Editor;
                                    status_msg = "Focused Editor".into();
                                } else if (380..455).contains(&mouse_x) {
                                    view_mode = WorkspaceMode::Split;
                                    status_msg = "Mode: Split View".into();
                                } else if (460..535).contains(&mouse_x) {
                                    view_mode = WorkspaceMode::EditorOnly;
                                    status_msg = "Mode: Editor Only".into();
                                } else if (540..620).contains(&mouse_x) {
                                    view_mode = WorkspaceMode::PreviewOnly;
                                    status_msg = "Mode: Preview Only".into();
                                }
                            } else if mouse_x < layout.sidebar_rect.right() {
                                focus = ActiveFocus::Sidebar;
                                let clicked_row = (mouse_y - (layout.sidebar_rect.y + 34)) / 24;
                                if clicked_row >= 0 && (clicked_row as usize) < nb_mgr.notebooks.len() {
                                    if is_dirty {
                                        let mut doc_bytes = vec![0u8; buffer.len()];
                                        buffer.copy_range(0, buffer.len(), &mut doc_bytes);
                                        let _ = nb_mgr.save_active_content(&doc_bytes);
                                        if let (Some(nb_path), Some(page_name)) = (nb_mgr.active_notebook_dir(), nb_mgr.pages.get(nb_mgr.active_page_idx)) {
                                            let _ = RecoveryJournal::discard_snapshot(&StdFs, &nb_path.display().to_string(), page_name);
                                        }
                                        is_dirty = false;
                                    }
                                    nb_mgr.active_notebook_idx = clicked_row as usize;
                                    nb_mgr.refresh_pages();
                                    doc_memory = nb_mgr.load_active_content();
                                    buffer = PieceTable::new(&doc_memory);
                                    cursor_pos = buffer.len();
                                    selection = Selection::collapsed(Cursor::new(cursor_pos, 0, 0));
                                    undo_mgr.clear();
                                    status_msg = format!("Selected notebook '{}'", nb_mgr.notebooks[nb_mgr.active_notebook_idx]);
                                }
                            } else if mouse_x < layout.page_list_rect.right() {
                                focus = ActiveFocus::PageList;
                                let clicked_row = (mouse_y - (layout.page_list_rect.y + 34)) / 24;
                                if clicked_row >= 0 && (clicked_row as usize) < nb_mgr.pages.len() {
                                    if is_dirty {
                                        let mut doc_bytes = vec![0u8; buffer.len()];
                                        buffer.copy_range(0, buffer.len(), &mut doc_bytes);
                                        let _ = nb_mgr.save_active_content(&doc_bytes);
                                        if let (Some(nb_path), Some(page_name)) = (nb_mgr.active_notebook_dir(), nb_mgr.pages.get(nb_mgr.active_page_idx)) {
                                            let _ = RecoveryJournal::discard_snapshot(&StdFs, &nb_path.display().to_string(), page_name);
                                        }
                                        is_dirty = false;
                                    }
                                    nb_mgr.active_page_idx = clicked_row as usize;
                                    doc_memory = nb_mgr.load_active_content();
                                    buffer = PieceTable::new(&doc_memory);
                                    cursor_pos = buffer.len();
                                    selection = Selection::collapsed(Cursor::new(cursor_pos, 0, 0));
                                    undo_mgr.clear();
                                    status_msg = format!("Selected page '{}'", nb_mgr.pages[nb_mgr.active_page_idx]);
                                }
                            } else if layout.editor_rect.width > 0 && mouse_x < layout.editor_rect.right() {
                                focus = ActiveFocus::Editor;
                                let gutter_w = 44;
                                let text_x = layout.editor_rect.x + gutter_w + 10;
                                let text_y = layout.editor_rect.y + 10;
                                let target_line = ((mouse_y - text_y + editor_scroll_y) / 18).max(0) as usize;
                                let target_col = ((mouse_x - text_x) / 8).max(0) as usize;
                                let off = offset_from_line_col(&buffer, target_line, target_col);
                                cursor_pos = off;
                                selection = Selection::collapsed(Cursor::new(off, target_line, target_col));
                                dragging_selection = true;
                                status_msg = "Focused Editor".into();
                            } else if layout.preview_rect.width > 0 && mouse_x >= layout.preview_rect.x {
                                // Hit-test interactive preview click targets (links, task checkboxes, headings)
                                let mut handled_target = false;
                                for target in &current_click_targets {
                                    match target {
                                        ClickableTarget::Link { rect, url } => {
                                            if rect.contains(mouse_x, mouse_y) {
                                                let _ = std::process::Command::new("xdg-open").arg(url).spawn();
                                                status_msg = format!("Opened link: {}", url);
                                                handled_target = true;
                                                break;
                                            }
                                        }
                                        ClickableTarget::Checkbox { rect, source_byte_offset, is_checked } => {
                                            if rect.contains(mouse_x, mouse_y) {
                                                let off = *source_byte_offset;
                                                let mut slice = [0u8; 6];
                                                let n = buffer.copy_range(off, 6, &mut slice);
                                                if n >= 5 {
                                                    let s = core::str::from_utf8(&slice[..5]).unwrap_or("");
                                                    if s.starts_with("- [ ]") || s.starts_with("- [x]") {
                                                        undo_mgr.push_undo(buffer.take_snapshot(cursor_pos));
                                                        buffer.delete(off, 5);
                                                        buffer.insert(off, if *is_checked { b"- [ ]" } else { b"- [x]" });
                                                        is_dirty = true;
                                                        last_edit_time = Instant::now();
                                                        status_msg = if *is_checked { "Unchecked task" } else { "Checked task" }.into();
                                                        handled_target = true;
                                                        break;
                                                    }
                                                }
                                            }
                                        }
                                        ClickableTarget::Heading { rect, source_byte_offset } => {
                                            if rect.contains(mouse_x, mouse_y) {
                                                cursor_pos = *source_byte_offset;
                                                selection = Selection::collapsed(Cursor::new(cursor_pos, 0, 0));
                                                focus = ActiveFocus::Editor;
                                                status_msg = "Navigated editor to heading".into();
                                                handled_target = true;
                                                break;
                                            }
                                        }
                                    }
                                }
                                if !handled_target {
                                    status_msg = "Preview Viewport".into();
                                }
                            }
                            current_click_targets = redraw(
                                &mut fb, width, height, sidebar_w, page_list_w, dragging_splitter,
                                &buffer, &mut parser, &nb_mgr, focus, cursor_pos, &selection,
                                editor_scroll_y, scroll_y, is_dirty, view_mode, &status_msg,
                            );
                            let _ = win.present_framebuffer(&fb, width, height);
                        } else if button == 4 {
                            // Scroll wheel UP with synchronized scroll locking
                            if mouse_x < layout.editor_rect.right() && mouse_x >= layout.editor_rect.x {
                                editor_scroll_y = (editor_scroll_y - 24).max(0);
                                let top_line = (editor_scroll_y / 18) as usize;
                                let off = buffer.line_to_offset(top_line);
                                let mut doc_bytes = vec![0u8; buffer.len()];
                                buffer.copy_range(0, buffer.len(), &mut doc_bytes);
                                let layouts = PreviewRenderer::compute_layouts(&parser.blocks, &doc_bytes, layout.preview_rect.width);
                                if let Some(bl) = layouts.iter().find(|l| off >= l.source_start && off <= l.source_end) {
                                    scroll_y = bl.y;
                                }
                            } else {
                                scroll_y = (scroll_y - 24).max(0);
                                let mut doc_bytes = vec![0u8; buffer.len()];
                                buffer.copy_range(0, buffer.len(), &mut doc_bytes);
                                let layouts = PreviewRenderer::compute_layouts(&parser.blocks, &doc_bytes, layout.preview_rect.width);
                                if let Some(bl) = layouts.iter().find(|l| l.y + l.height >= scroll_y) {
                                    editor_scroll_y = (buffer.offset_to_line(bl.source_start).0 as i32) * 18;
                                }
                            }
                            current_click_targets = redraw(
                                &mut fb, width, height, sidebar_w, page_list_w, dragging_splitter,
                                &buffer, &mut parser, &nb_mgr, focus, cursor_pos, &selection,
                                editor_scroll_y, scroll_y, is_dirty, view_mode, &status_msg,
                            );
                            let _ = win.present_framebuffer(&fb, width, height);
                        } else if button == 5 {
                            // Scroll wheel DOWN with synchronized scroll locking
                            if mouse_x < layout.editor_rect.right() && mouse_x >= layout.editor_rect.x {
                                editor_scroll_y += 24;
                                let top_line = (editor_scroll_y / 18) as usize;
                                let off = buffer.line_to_offset(top_line);
                                let mut doc_bytes = vec![0u8; buffer.len()];
                                buffer.copy_range(0, buffer.len(), &mut doc_bytes);
                                let layouts = PreviewRenderer::compute_layouts(&parser.blocks, &doc_bytes, layout.preview_rect.width);
                                if let Some(bl) = layouts.iter().find(|l| off >= l.source_start && off <= l.source_end) {
                                    scroll_y = bl.y;
                                }
                            } else {
                                scroll_y += 24;
                                let mut doc_bytes = vec![0u8; buffer.len()];
                                buffer.copy_range(0, buffer.len(), &mut doc_bytes);
                                let layouts = PreviewRenderer::compute_layouts(&parser.blocks, &doc_bytes, layout.preview_rect.width);
                                if let Some(bl) = layouts.iter().find(|l| l.y + l.height >= scroll_y) {
                                    editor_scroll_y = (buffer.offset_to_line(bl.source_start).0 as i32) * 18;
                                }
                            }
                            current_click_targets = redraw(
                                &mut fb, width, height, sidebar_w, page_list_w, dragging_splitter,
                                &buffer, &mut parser, &nb_mgr, focus, cursor_pos, &selection,
                                editor_scroll_y, scroll_y, is_dirty, view_mode, &status_msg,
                            );
                            let _ = win.present_framebuffer(&fb, width, height);
                        }
                    }
                    // KeyPress event (opcode 2): Real X11 Keyboard & Unicode Engine!
                    2 => {
                        let keycode = event[1];
                        let state = u16::from_le_bytes([event[28], event[29]]);
                        let action = win.translate_key(keycode, state);

                        match action {
                            KeyAction::Escape => {
                                println!("\nEscape pressed. Exiting Lekhni.");
                                break;
                            }
                            KeyAction::F1 => {
                                view_mode = WorkspaceMode::Split;
                                status_msg = "Switched to Split View".into();
                            }
                            KeyAction::F2 => {
                                view_mode = WorkspaceMode::EditorOnly;
                                status_msg = "Switched to Editor Only".into();
                            }
                            KeyAction::F3 => {
                                view_mode = WorkspaceMode::PreviewOnly;
                                status_msg = "Switched to Preview Only".into();
                            }
                            KeyAction::Tab => {
                                focus = match focus {
                                    ActiveFocus::Sidebar => ActiveFocus::PageList,
                                    ActiveFocus::PageList => ActiveFocus::Editor,
                                    ActiveFocus::Editor => ActiveFocus::Sidebar,
                                };
                                status_msg = format!("Focused: {:?}", focus);
                            }
                            KeyAction::Save => {
                                let mut doc_bytes = vec![0u8; buffer.len()];
                                buffer.copy_range(0, buffer.len(), &mut doc_bytes);
                                match nb_mgr.save_active_content(&doc_bytes) {
                                    Ok(()) => {
                                        if let (Some(nb_path), Some(page_name)) = (nb_mgr.active_notebook_dir(), nb_mgr.pages.get(nb_mgr.active_page_idx)) {
                                            let _ = RecoveryJournal::discard_snapshot(&StdFs, &nb_path.display().to_string(), page_name);
                                        }
                                        is_dirty = false;
                                        status_msg = "Saved successfully!".into();
                                    }
                                    Err(e) => {
                                        status_msg = format!("Save error: {}", e);
                                    }
                                }
                            }
                            KeyAction::NewNotebook => {
                                if is_dirty {
                                    let mut doc_bytes = vec![0u8; buffer.len()];
                                    buffer.copy_range(0, buffer.len(), &mut doc_bytes);
                                    let _ = nb_mgr.save_active_content(&doc_bytes);
                                }
                                let new_name = nb_mgr.create_notebook("Notebook");
                                doc_memory = nb_mgr.load_active_content();
                                buffer = PieceTable::new(&doc_memory);
                                cursor_pos = buffer.len();
                                selection = Selection::collapsed(Cursor::new(cursor_pos, 0, 0));
                                undo_mgr.clear();
                                is_dirty = false;
                                status_msg = format!("Created & opened notebook '{}'", new_name);
                            }
                            KeyAction::NewPage => {
                                if is_dirty {
                                    let mut doc_bytes = vec![0u8; buffer.len()];
                                    buffer.copy_range(0, buffer.len(), &mut doc_bytes);
                                    let _ = nb_mgr.save_active_content(&doc_bytes);
                                }
                                let new_page_name = format!("note_{}.md", nb_mgr.pages.len() + 1);
                                nb_mgr.create_page(&new_page_name, b"# New Note\n\nStart typing...\n");
                                doc_memory = nb_mgr.load_active_content();
                                buffer = PieceTable::new(&doc_memory);
                                cursor_pos = buffer.len();
                                selection = Selection::collapsed(Cursor::new(cursor_pos, 0, 0));
                                undo_mgr.clear();
                                is_dirty = false;
                                status_msg = format!("Created & opened page '{}'", new_page_name);
                            }
                            KeyAction::Undo => {
                                if focus == ActiveFocus::Editor {
                                    if let Some(prev) = undo_mgr.undo(buffer.take_snapshot(cursor_pos)) {
                                        cursor_pos = prev.cursor_offset.min(prev.total_len);
                                        selection = Selection::collapsed(Cursor::new(cursor_pos, 0, 0));
                                        buffer.restore_snapshot(prev);
                                        is_dirty = true;
                                        last_edit_time = Instant::now();
                                        status_msg = "Undo".into();
                                    }
                                }
                            }
                            KeyAction::Redo => {
                                if focus == ActiveFocus::Editor {
                                    if let Some(next) = undo_mgr.redo(buffer.take_snapshot(cursor_pos)) {
                                        cursor_pos = next.cursor_offset.min(next.total_len);
                                        selection = Selection::collapsed(Cursor::new(cursor_pos, 0, 0));
                                        buffer.restore_snapshot(next);
                                        is_dirty = true;
                                        last_edit_time = Instant::now();
                                        status_msg = "Redo".into();
                                    }
                                }
                            }
                            KeyAction::Copy => {
                                if focus == ActiveFocus::Editor && !selection.is_collapsed() {
                                    SelectionHandler::copy_selection(&selection, &buffer, &mut clipboard);
                                    status_msg = "Copied to clipboard".into();
                                }
                            }
                            KeyAction::Cut => {
                                if focus == ActiveFocus::Editor && !selection.is_collapsed() {
                                    SelectionHandler::cut_selection(&mut selection, &mut buffer, &mut undo_mgr, &mut clipboard);
                                    cursor_pos = selection.head.byte_offset;
                                    is_dirty = true;
                                    last_edit_time = Instant::now();
                                    status_msg = "Cut to clipboard".into();
                                }
                            }
                            KeyAction::Paste => {
                                if focus == ActiveFocus::Editor {
                                    SelectionHandler::paste_clipboard(&mut selection, &mut buffer, &mut undo_mgr, &clipboard);
                                    cursor_pos = selection.head.byte_offset;
                                    is_dirty = true;
                                    last_edit_time = Instant::now();
                                    status_msg = "Pasted from clipboard".into();
                                }
                            }
                            KeyAction::SelectAll => {
                                if focus == ActiveFocus::Editor {
                                    selection.anchor = Cursor::new(0, 0, 0);
                                    selection.head = Cursor::new(buffer.len(), 0, 0);
                                    cursor_pos = buffer.len();
                                    status_msg = "Selected all".into();
                                }
                            }
                            KeyAction::Left { shift, .. } => {
                                if focus == ActiveFocus::Editor {
                                    let new_pos = prev_char_boundary(&buffer, cursor_pos);
                                    if shift {
                                        selection.head.byte_offset = new_pos;
                                    } else {
                                        selection = Selection::collapsed(Cursor::new(new_pos, 0, 0));
                                    }
                                    cursor_pos = new_pos;
                                }
                            }
                            KeyAction::Right { shift, .. } => {
                                if focus == ActiveFocus::Editor {
                                    let new_pos = next_char_boundary(&buffer, cursor_pos);
                                    if shift {
                                        selection.head.byte_offset = new_pos;
                                    } else {
                                        selection = Selection::collapsed(Cursor::new(new_pos, 0, 0));
                                    }
                                    cursor_pos = new_pos;
                                }
                            }
                            KeyAction::Up { shift } => {
                                match focus {
                                    ActiveFocus::Sidebar => {
                                        if nb_mgr.active_notebook_idx > 0 {
                                            nb_mgr.active_notebook_idx -= 1;
                                            nb_mgr.refresh_pages();
                                            doc_memory = nb_mgr.load_active_content();
                                            buffer = PieceTable::new(&doc_memory);
                                            cursor_pos = buffer.len();
                                            selection = Selection::collapsed(Cursor::new(cursor_pos, 0, 0));
                                            status_msg = "Switched notebook".into();
                                        }
                                    }
                                    ActiveFocus::PageList => {
                                        if nb_mgr.active_page_idx > 0 {
                                            nb_mgr.active_page_idx -= 1;
                                            doc_memory = nb_mgr.load_active_content();
                                            buffer = PieceTable::new(&doc_memory);
                                            cursor_pos = buffer.len();
                                            selection = Selection::collapsed(Cursor::new(cursor_pos, 0, 0));
                                            status_msg = "Switched page".into();
                                        }
                                    }
                                    ActiveFocus::Editor => {
                                        let (line, line_start) = buffer.offset_to_line(cursor_pos);
                                        let col = cursor_pos.saturating_sub(line_start);
                                        if line > 0 {
                                            let new_pos = offset_from_line_col(&buffer, line - 1, col);
                                            if shift {
                                                selection.head.byte_offset = new_pos;
                                            } else {
                                                selection = Selection::collapsed(Cursor::new(new_pos, 0, 0));
                                            }
                                            cursor_pos = new_pos;
                                        }
                                    }
                                }
                            }
                            KeyAction::Down { shift } => {
                                match focus {
                                    ActiveFocus::Sidebar => {
                                        if nb_mgr.active_notebook_idx + 1 < nb_mgr.notebooks.len() {
                                            nb_mgr.active_notebook_idx += 1;
                                            nb_mgr.refresh_pages();
                                            doc_memory = nb_mgr.load_active_content();
                                            buffer = PieceTable::new(&doc_memory);
                                            cursor_pos = buffer.len();
                                            selection = Selection::collapsed(Cursor::new(cursor_pos, 0, 0));
                                            status_msg = "Switched notebook".into();
                                        }
                                    }
                                    ActiveFocus::PageList => {
                                        if nb_mgr.active_page_idx + 1 < nb_mgr.pages.len() {
                                            nb_mgr.active_page_idx += 1;
                                            doc_memory = nb_mgr.load_active_content();
                                            buffer = PieceTable::new(&doc_memory);
                                            cursor_pos = buffer.len();
                                            selection = Selection::collapsed(Cursor::new(cursor_pos, 0, 0));
                                            status_msg = "Switched page".into();
                                        }
                                    }
                                    ActiveFocus::Editor => {
                                        let (line, line_start) = buffer.offset_to_line(cursor_pos);
                                        let col = cursor_pos.saturating_sub(line_start);
                                        let new_pos = offset_from_line_col(&buffer, line + 1, col);
                                        if shift {
                                            selection.head.byte_offset = new_pos;
                                        } else {
                                            selection = Selection::collapsed(Cursor::new(new_pos, 0, 0));
                                        }
                                        cursor_pos = new_pos;
                                    }
                                }
                            }
                            KeyAction::Home { shift } => {
                                if focus == ActiveFocus::Editor {
                                    let (_, line_start) = buffer.offset_to_line(cursor_pos);
                                    let new_pos = line_start;
                                    if shift {
                                        selection.head.byte_offset = new_pos;
                                    } else {
                                        selection = Selection::collapsed(Cursor::new(new_pos, 0, 0));
                                    }
                                    cursor_pos = new_pos;
                                }
                            }
                            KeyAction::End { shift } => {
                                if focus == ActiveFocus::Editor {
                                    let new_pos = find_line_end(&buffer, cursor_pos);
                                    if shift {
                                        selection.head.byte_offset = new_pos;
                                    } else {
                                        selection = Selection::collapsed(Cursor::new(new_pos, 0, 0));
                                    }
                                    cursor_pos = new_pos;
                                }
                            }
                            KeyAction::PageUp => {
                                editor_scroll_y = (editor_scroll_y - 200).max(0);
                            }
                            KeyAction::PageDown => {
                                editor_scroll_y += 200;
                            }
                            KeyAction::Backspace => {
                                if focus == ActiveFocus::Editor {
                                    undo_mgr.push_undo(buffer.take_snapshot(cursor_pos));
                                    if !selection.is_collapsed() {
                                        let (s, e) = selection.byte_range();
                                        buffer.delete(s, e - s);
                                        cursor_pos = s;
                                        selection = Selection::collapsed(Cursor::new(s, 0, 0));
                                    } else if cursor_pos > 0 && !buffer.is_empty() {
                                        let prev = prev_char_boundary(&buffer, cursor_pos);
                                        buffer.delete(prev, cursor_pos - prev);
                                        cursor_pos = prev;
                                        selection = Selection::collapsed(Cursor::new(cursor_pos, 0, 0));
                                    }
                                    is_dirty = true;
                                    last_edit_time = Instant::now();
                                    status_msg = "Edited".into();
                                }
                            }
                            KeyAction::Delete => {
                                if focus == ActiveFocus::Editor {
                                    undo_mgr.push_undo(buffer.take_snapshot(cursor_pos));
                                    if !selection.is_collapsed() {
                                        let (s, e) = selection.byte_range();
                                        buffer.delete(s, e - s);
                                        cursor_pos = s;
                                        selection = Selection::collapsed(Cursor::new(s, 0, 0));
                                    } else if cursor_pos < buffer.len() {
                                        let next = next_char_boundary(&buffer, cursor_pos);
                                        buffer.delete(cursor_pos, next - cursor_pos);
                                        selection = Selection::collapsed(Cursor::new(cursor_pos, 0, 0));
                                    }
                                    is_dirty = true;
                                    last_edit_time = Instant::now();
                                    status_msg = "Edited".into();
                                }
                            }
                            KeyAction::Return => {
                                match focus {
                                    ActiveFocus::Sidebar | ActiveFocus::PageList => {
                                        focus = ActiveFocus::Editor;
                                        status_msg = "Focused Editor".into();
                                    }
                                    ActiveFocus::Editor => {
                                        undo_mgr.push_undo(buffer.take_snapshot(cursor_pos));
                                        if !selection.is_collapsed() {
                                            let (s, e) = selection.byte_range();
                                            buffer.delete(s, e - s);
                                            cursor_pos = s;
                                        }
                                        buffer.insert(cursor_pos, b"\n");
                                        cursor_pos += 1;
                                        selection = Selection::collapsed(Cursor::new(cursor_pos, 0, 0));
                                        is_dirty = true;
                                        last_edit_time = Instant::now();
                                        status_msg = "Edited".into();
                                    }
                                }
                            }
                            KeyAction::Char(c) => {
                                if focus == ActiveFocus::Editor {
                                    undo_mgr.push_undo(buffer.take_snapshot(cursor_pos));
                                    if !selection.is_collapsed() {
                                        let (s, e) = selection.byte_range();
                                        buffer.delete(s, e - s);
                                        cursor_pos = s;
                                    }
                                    let mut utf8_buf = [0u8; 4];
                                    let s = c.encode_utf8(&mut utf8_buf);
                                    buffer.insert(cursor_pos, s.as_bytes());
                                    cursor_pos += s.len();
                                    selection = Selection::collapsed(Cursor::new(cursor_pos, 0, 0));
                                    is_dirty = true;
                                    last_edit_time = Instant::now();
                                    status_msg = "Edited".into();
                                }
                            }
                            KeyAction::None => {}
                        }

                        // Write recovery journal immediately on edit
                        if is_dirty {
                            if let (Some(nb_path), Some(page_name)) = (nb_mgr.active_notebook_dir(), nb_mgr.pages.get(nb_mgr.active_page_idx)) {
                                let mut dirty_bytes = vec![0u8; buffer.len()];
                                buffer.copy_range(0, buffer.len(), &mut dirty_bytes);
                                let _ = RecoveryJournal::write_snapshot(&StdFs, &nb_path.display().to_string(), page_name, &dirty_bytes);
                            }
                        }

                        // Auto-scroll editor so caret stays visible on screen
                        let layout = compute_workspace_layout_with_mode(
                            width as i32, height as i32, sidebar_w, page_list_w, 3, view_mode, 28, 24,
                        );
                        let (caret_line, _) = buffer.offset_to_line(cursor_pos);
                        let caret_y = (caret_line as i32) * 18;
                        let visible_h = (layout.editor_rect.height - 24).max(18);
                        if caret_y < editor_scroll_y {
                            editor_scroll_y = caret_y;
                        } else if caret_y + 18 > editor_scroll_y + visible_h {
                            editor_scroll_y = (caret_y + 18 - visible_h).max(0);
                        }

                        current_click_targets = redraw(
                            &mut fb, width, height, sidebar_w, page_list_w, dragging_splitter,
                            &buffer, &mut parser, &nb_mgr, focus, cursor_pos, &selection,
                            editor_scroll_y, scroll_y, is_dirty, view_mode, &status_msg,
                        );
                        let _ = win.present_framebuffer(&fb, width, height);
                    }
                    _ => {}
                }
            }
            Ok(None) => {
                // Timeout elapsed with no incoming event; loop iterates to handle autosave and stdin
            }
            Err(e) => {
                eprintln!("Display connection notice: {}", e);
                break;
            }
        }
    }
}
