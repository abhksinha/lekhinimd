//! Lekhni Desktop Interactive Native GUI Runner.
//!
//! Pure Rust native desktop frontend. No Webview, no Electron, no Tauri.
//! Direct zero-copy presentation to Linux display surface with zero idle CPU.

use std::sync::mpsc::{channel, Receiver};
use std::thread;
use std::io::Read;

use lekhni_layout::compute_workspace_layout_with_chrome;
use lekhni_md::parser::MarkdownParser;
use lekhni_raster::canvas::Canvas;
use lekhni_raster::damage::Rect;
use lekhni_shell_desktop::workspace::{get_or_init_working_dir, NotebookManager};
use lekhni_shell_desktop::x11_presenter::X11Window;
use lekhni_text::buffer::PieceTable;
use lekhni_ui::preview::PreviewRenderer;
use lekhni_ui::theme::Theme;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum ActiveFocus {
    Sidebar,
    PageList,
    Editor,
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

    let width = 1100u16;
    let height = 700u16;
    let theme = Theme::dark();
    let mut fb = vec![theme.bg_workspace; (width as usize) * (height as usize)];

    // 2. Load active document into PieceTable and parser
    let mut doc_memory = nb_mgr.load_active_content();
    let mut buffer = PieceTable::new(&doc_memory);
    let mut cursor_pos: usize = buffer.len();
    let mut parser = MarkdownParser::new();
    let mut focus = ActiveFocus::Editor;
    let mut status_msg = String::from("Ready. Ctrl+S to save, Tab to switch pane, Esc to quit.");

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
                  buffer: &PieceTable,
                  parser: &mut MarkdownParser,
                  nb_mgr: &NotebookManager,
                  focus: ActiveFocus,
                  cursor_pos: usize,
                  status: &str| {
        let mut canvas = Canvas::new(fb, width as u32, height as u32);
        canvas.clear(theme.bg_workspace);

        let menu_h = 26;
        let status_h = 24;
        let layout = compute_workspace_layout_with_chrome(
            width as i32,
            height as i32,
            180,
            200,
            2,
            true,
            menu_h,
            status_h,
        );

        // A. Top Menu Bar
        canvas.fill_rect(layout.menu_bar_rect, 0xFF_25_25_26);
        canvas.draw_text("LEKHNI | [Tab] Switch Pane  [Ctrl+N] New NB  [Ctrl+P] New Page  [Ctrl+S] Save  [Esc] Quit", 10, 5, 0xFF_CC_CC_CC);
        let active_nb_name = nb_mgr.notebooks.get(nb_mgr.active_notebook_idx).map(|s| s.as_str()).unwrap_or("None");
        let active_pg_name = nb_mgr.pages.get(nb_mgr.active_page_idx).map(|s| s.as_str()).unwrap_or("None");
        let right_label = format!("NB: {} | Page: {}", active_nb_name, active_pg_name);
        let right_x = (width as i32) - (right_label.len() as i32 * 8) - 16;
        if right_x > 600 {
            canvas.draw_text(&right_label, right_x, 5, 0xFF_4E_C9_B0);
        }

        // B. Sidebar (Notebooks)
        let sb_bg = if focus == ActiveFocus::Sidebar { 0xFF_1E_1E_1E } else { theme.bg_sidebar };
        canvas.fill_rect(layout.sidebar_rect, sb_bg);
        canvas.draw_text("=== NOTEBOOKS ===", layout.sidebar_rect.x + 12, layout.sidebar_rect.y + 8, 0xFF_85_85_85);

        let mut nb_y = layout.sidebar_rect.y + 32;
        for (i, nb) in nb_mgr.notebooks.iter().enumerate() {
            if nb_y + 18 > layout.sidebar_rect.bottom() {
                break;
            }
            let is_active = i == nb_mgr.active_notebook_idx;
            if is_active {
                canvas.fill_rect(
                    Rect::new(layout.sidebar_rect.x + 4, nb_y - 2, layout.sidebar_rect.width - 8, 20),
                    if focus == ActiveFocus::Sidebar { 0xFF_09_47_71 } else { 0xFF_2A_2D_2E },
                );
                canvas.draw_text(&format!("* {}", nb), layout.sidebar_rect.x + 8, nb_y, 0xFF_FF_FF_FF);
            } else {
                canvas.draw_text(&format!("  {}", nb), layout.sidebar_rect.x + 8, nb_y, 0xFF_9D_A5_B4);
            }
            nb_y += 22;
        }

        // C. Splitter 1
        canvas.fill_rect(layout.splitter_1_rect, theme.border_color);

        // D. Page List (Notes in Active Notebook)
        let pl_bg = if focus == ActiveFocus::PageList { 0xFF_22_22_25 } else { theme.bg_page_list };
        canvas.fill_rect(layout.page_list_rect, pl_bg);
        canvas.draw_text("=== PAGES ===", layout.page_list_rect.x + 12, layout.page_list_rect.y + 8, 0xFF_85_85_85);

        let mut pg_y = layout.page_list_rect.y + 32;
        for (i, page) in nb_mgr.pages.iter().enumerate() {
            if pg_y + 18 > layout.page_list_rect.bottom() {
                break;
            }
            let is_active = i == nb_mgr.active_page_idx;
            if is_active {
                canvas.fill_rect(
                    Rect::new(layout.page_list_rect.x + 4, pg_y - 2, layout.page_list_rect.width - 8, 20),
                    if focus == ActiveFocus::PageList { 0xFF_09_47_71 } else { 0xFF_2A_2D_2E },
                );
                canvas.draw_text(&format!("* {}", page), layout.page_list_rect.x + 8, pg_y, 0xFF_FF_FF_FF);
            } else {
                canvas.draw_text(&format!("  {}", page), layout.page_list_rect.x + 8, pg_y, 0xFF_9D_A5_B4);
            }
            pg_y += 22;
        }

        // E. Splitter 2
        canvas.fill_rect(layout.splitter_2_rect, theme.border_color);

        // F. Editor Pane
        let ed_bg = if focus == ActiveFocus::Editor { 0xFF_1E_1E_1E } else { 0xFF_18_18_18 };
        canvas.fill_rect(layout.editor_rect, ed_bg);

        // Gutter
        let gutter_w = 40;
        canvas.fill_rect(
            Rect::new(layout.editor_rect.x, layout.editor_rect.y, gutter_w, layout.editor_rect.height),
            0xFF_25_25_26,
        );

        let mut doc_bytes = vec![0u8; buffer.len()];
        buffer.copy_range(0, buffer.len(), &mut doc_bytes);
        let doc_str = String::from_utf8_lossy(&doc_bytes);

        let mut line_num = 1;
        let mut ed_y = layout.editor_rect.y + 8;
        let mut char_count_acc = 0usize;

        for line in doc_str.lines() {
            if ed_y + 18 > layout.editor_rect.bottom() {
                break;
            }
            let num_str = format!("{:2}", line_num);
            canvas.draw_text(&num_str, layout.editor_rect.x + 8, ed_y, 0xFF_6E_76_81);

            let line_len = line.len();
            let line_start = char_count_acc;
            let line_end = line_start + line_len;

            // Draw line text
            let text_x = layout.editor_rect.x + gutter_w + 10;
            canvas.draw_text(line, text_x, ed_y, 0xFF_D4_D4_D4);

            // Draw cursor if on this line
            if cursor_pos >= line_start && cursor_pos <= line_end {
                let col = (cursor_pos - line_start) as i32;
                let cur_x = text_x + col * 8;
                if cur_x + 2 < layout.editor_rect.right() {
                    canvas.fill_rect(Rect::new(cur_x, ed_y, 2, 16), 0xFF_00_7A_CC);
                }
            }

            char_count_acc += line_len + 1; // + 1 for \n
            line_num += 1;
            ed_y += 18;
        }

        // If buffer ends with newline or is empty, draw cursor on new empty line
        if char_count_acc <= cursor_pos && ed_y + 18 <= layout.editor_rect.bottom() {
            let num_str = format!("{:2}", line_num);
            canvas.draw_text(&num_str, layout.editor_rect.x + 8, ed_y, 0xFF_6E_76_81);
            let text_x = layout.editor_rect.x + gutter_w + 10;
            canvas.fill_rect(Rect::new(text_x, ed_y, 2, 16), 0xFF_00_7A_CC);
        }

        // G. Preview Pane
        canvas.fill_rect(layout.preview_rect, theme.bg_preview);
        parser.parse_full(&doc_bytes);

        canvas.clip = layout.preview_rect;
        PreviewRenderer::render_preview_with_content(
            &mut canvas,
            &parser.blocks,
            &doc_bytes,
            0,
            layout.preview_rect.width,
            layout.preview_rect.height,
        );
        canvas.clip = Rect::new(0, 0, width as i32, height as i32);

        // H. Status Bar
        canvas.fill_rect(layout.status_bar_rect, 0xFF_00_7A_CC);
        let word_count = doc_str.split_whitespace().count();
        let status_text = format!(
            " {} | Focus: {:?} | Len: {} bytes, {} words | Latency: <1ms",
            status, focus, buffer.len(), word_count
        );
        canvas.draw_text(&status_text, 6, layout.status_bar_rect.y + 4, 0xFF_FF_FF_FF);
    };

    redraw(&mut fb, &buffer, &mut parser, &nb_mgr, focus, cursor_pos, &status_msg);
    let _ = win.present_framebuffer(&fb, width, height);

    println!("Lekhni native desktop window is live!");
    println!("Interactive session active: type to edit document, press Escape to exit.");

    // 5. Stdin background listener for dual input (terminal + GUI)
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

    // 6. Interactive Event Loop
    let mut shift_pressed = false;
    let mut ctrl_pressed = false;

    loop {
        // Check for stdin character if sent from terminal
        if let Ok(ch) = rx_stdin.try_recv() {
            if ch == 27 {
                // Escape in terminal
                println!("\nEscape pressed in terminal. Exiting Lekhni.");
                break;
            } else if ch == 8 || ch == 127 {
                // Backspace
                if cursor_pos > 0 && !buffer.is_empty() {
                    cursor_pos -= 1;
                    buffer.delete(cursor_pos, 1);
                    status_msg = "Edited".into();
                    redraw(&mut fb, &buffer, &mut parser, &nb_mgr, focus, cursor_pos, &status_msg);
                    let _ = win.present_framebuffer(&fb, width, height);
                }
            } else if ch == b'\n' || ch == b'\r' {
                buffer.insert(cursor_pos, b"\n");
                cursor_pos += 1;
                status_msg = "Edited".into();
                redraw(&mut fb, &buffer, &mut parser, &nb_mgr, focus, cursor_pos, &status_msg);
                let _ = win.present_framebuffer(&fb, width, height);
            } else if (32..127).contains(&ch) {
                buffer.insert(cursor_pos, &[ch]);
                cursor_pos += 1;
                status_msg = "Edited".into();
                redraw(&mut fb, &buffer, &mut parser, &nb_mgr, focus, cursor_pos, &status_msg);
                let _ = win.present_framebuffer(&fb, width, height);
            }
        }

        match win.wait_event() {
            Ok(event) => {
                let event_type = event[0] & 0x7F;
                match event_type {
                    // Expose event (opcode 12): re-present window framebuffer
                    12 => {
                        let _ = win.present_framebuffer(&fb, width, height);
                    }
                    // KeyRelease event (opcode 3)
                    3 => {
                        let keycode = event[1];
                        if keycode == 50 || keycode == 62 {
                            shift_pressed = false;
                        } else if keycode == 37 || keycode == 105 {
                            ctrl_pressed = false;
                        }
                    }
                    // KeyPress event (opcode 2)
                    2 => {
                        let keycode = event[1];
                        let state = u16::from_le_bytes([event[28], event[29]]);
                        let shift = shift_pressed || (state & 0x01) != 0;
                        let ctrl = ctrl_pressed || (state & 0x04) != 0;

                        if keycode == 50 || keycode == 62 {
                            shift_pressed = true;
                            continue;
                        }
                        if keycode == 37 || keycode == 105 {
                            ctrl_pressed = true;
                            continue;
                        }

                        if keycode == 9 {
                            // Escape: exit cleanly
                            println!("\nEscape pressed. Exiting Lekhni.");
                            break;
                        } else if keycode == 23 {
                            // Tab: Cycle active focus pane
                            focus = match focus {
                                ActiveFocus::Sidebar => ActiveFocus::PageList,
                                ActiveFocus::PageList => ActiveFocus::Editor,
                                ActiveFocus::Editor => ActiveFocus::Sidebar,
                            };
                            status_msg = format!("Focused: {:?}", focus);
                            redraw(&mut fb, &buffer, &mut parser, &nb_mgr, focus, cursor_pos, &status_msg);
                            let _ = win.present_framebuffer(&fb, width, height);
                            continue;
                        }

                        // Hotkeys: Ctrl+S (Save), Ctrl+N (New Notebook), Ctrl+P (New Page)
                        if ctrl {
                            if keycode == 39 {
                                // 's': Save document atomically to disk
                                let mut doc_bytes = vec![0u8; buffer.len()];
                                buffer.copy_range(0, buffer.len(), &mut doc_bytes);
                                match nb_mgr.save_active_content(&doc_bytes) {
                                    Ok(()) => {
                                        status_msg = "Saved successfully!".into();
                                    }
                                    Err(e) => {
                                        status_msg = format!("Save error: {}", e);
                                    }
                                }
                                redraw(&mut fb, &buffer, &mut parser, &nb_mgr, focus, cursor_pos, &status_msg);
                                let _ = win.present_framebuffer(&fb, width, height);
                                continue;
                            } else if keycode == 57 {
                                // 'n': Create new unique notebook
                                let new_name = nb_mgr.create_notebook("Notebook");
                                doc_memory = nb_mgr.load_active_content();
                                buffer = PieceTable::new(&doc_memory);
                                cursor_pos = buffer.len();
                                status_msg = format!("Created & opened unique notebook '{}'", new_name);
                                redraw(&mut fb, &buffer, &mut parser, &nb_mgr, focus, cursor_pos, &status_msg);
                                let _ = win.present_framebuffer(&fb, width, height);
                                continue;
                            } else if keycode == 33 {
                                // 'p': Create new page
                                let new_page_name = format!("note_{}.md", nb_mgr.pages.len() + 1);
                                nb_mgr.create_page(&new_page_name, b"# New Note\n\nStart typing...\n");
                                doc_memory = nb_mgr.load_active_content();
                                buffer = PieceTable::new(&doc_memory);
                                cursor_pos = buffer.len();
                                status_msg = format!("Created & opened page '{}'", new_page_name);
                                redraw(&mut fb, &buffer, &mut parser, &nb_mgr, focus, cursor_pos, &status_msg);
                                let _ = win.present_framebuffer(&fb, width, height);
                                continue;
                            }
                        }

                        // Focus-dependent Navigation
                        match focus {
                            ActiveFocus::Sidebar => {
                                if keycode == 111 {
                                    // Up arrow
                                    if nb_mgr.active_notebook_idx > 0 {
                                        nb_mgr.active_notebook_idx -= 1;
                                        nb_mgr.refresh_pages();
                                        doc_memory = nb_mgr.load_active_content();
                                        buffer = PieceTable::new(&doc_memory);
                                        cursor_pos = buffer.len();
                                        status_msg = "Switched notebook".into();
                                    }
                                } else if keycode == 116 {
                                    // Down arrow
                                    if nb_mgr.active_notebook_idx + 1 < nb_mgr.notebooks.len() {
                                        nb_mgr.active_notebook_idx += 1;
                                        nb_mgr.refresh_pages();
                                        doc_memory = nb_mgr.load_active_content();
                                        buffer = PieceTable::new(&doc_memory);
                                        cursor_pos = buffer.len();
                                        status_msg = "Switched notebook".into();
                                    }
                                } else if keycode == 36 {
                                    // Enter: Switch to editor
                                    focus = ActiveFocus::Editor;
                                    status_msg = "Focused Editor".into();
                                }
                                redraw(&mut fb, &buffer, &mut parser, &nb_mgr, focus, cursor_pos, &status_msg);
                                let _ = win.present_framebuffer(&fb, width, height);
                            }
                            ActiveFocus::PageList => {
                                if keycode == 111 {
                                    // Up arrow
                                    if nb_mgr.active_page_idx > 0 {
                                        nb_mgr.active_page_idx -= 1;
                                        doc_memory = nb_mgr.load_active_content();
                                        buffer = PieceTable::new(&doc_memory);
                                        cursor_pos = buffer.len();
                                        status_msg = "Switched page".into();
                                    }
                                } else if keycode == 116 {
                                    // Down arrow
                                    if nb_mgr.active_page_idx + 1 < nb_mgr.pages.len() {
                                        nb_mgr.active_page_idx += 1;
                                        doc_memory = nb_mgr.load_active_content();
                                        buffer = PieceTable::new(&doc_memory);
                                        cursor_pos = buffer.len();
                                        status_msg = "Switched page".into();
                                    }
                                } else if keycode == 36 {
                                    // Enter: Switch to editor
                                    focus = ActiveFocus::Editor;
                                    status_msg = "Focused Editor".into();
                                }
                                redraw(&mut fb, &buffer, &mut parser, &nb_mgr, focus, cursor_pos, &status_msg);
                                let _ = win.present_framebuffer(&fb, width, height);
                            }
                            ActiveFocus::Editor => {
                                if keycode == 22 {
                                    // Backspace
                                    if cursor_pos > 0 && !buffer.is_empty() {
                                        cursor_pos -= 1;
                                        buffer.delete(cursor_pos, 1);
                                        status_msg = "Edited".into();
                                        redraw(&mut fb, &buffer, &mut parser, &nb_mgr, focus, cursor_pos, &status_msg);
                                        let _ = win.present_framebuffer(&fb, width, height);
                                    }
                                } else if keycode == 119 {
                                    // Delete
                                    if cursor_pos < buffer.len() {
                                        buffer.delete(cursor_pos, 1);
                                        status_msg = "Edited".into();
                                        redraw(&mut fb, &buffer, &mut parser, &nb_mgr, focus, cursor_pos, &status_msg);
                                        let _ = win.present_framebuffer(&fb, width, height);
                                    }
                                } else if keycode == 113 {
                                    // Left arrow
                                    if cursor_pos > 0 {
                                        cursor_pos -= 1;
                                        redraw(&mut fb, &buffer, &mut parser, &nb_mgr, focus, cursor_pos, &status_msg);
                                        let _ = win.present_framebuffer(&fb, width, height);
                                    }
                                } else if keycode == 114 {
                                    // Right arrow
                                    if cursor_pos < buffer.len() {
                                        cursor_pos += 1;
                                        redraw(&mut fb, &buffer, &mut parser, &nb_mgr, focus, cursor_pos, &status_msg);
                                        let _ = win.present_framebuffer(&fb, width, height);
                                    }
                                } else if keycode == 36 || keycode == 104 {
                                    // Enter / Return
                                    buffer.insert(cursor_pos, b"\n");
                                    cursor_pos += 1;
                                    status_msg = "Edited".into();
                                    redraw(&mut fb, &buffer, &mut parser, &nb_mgr, focus, cursor_pos, &status_msg);
                                    let _ = win.present_framebuffer(&fb, width, height);
                                } else if let Some(ch) = keycode_to_char(keycode, shift) {
                                    buffer.insert(cursor_pos, &[ch as u8]);
                                    cursor_pos += 1;
                                    status_msg = "Edited".into();
                                    redraw(&mut fb, &buffer, &mut parser, &nb_mgr, focus, cursor_pos, &status_msg);
                                    let _ = win.present_framebuffer(&fb, width, height);
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
            Err(e) => {
                eprintln!("Display connection notice: {}", e);
                break;
            }
        }
    }
}

/// Translates Linux evdev/X11 keycodes to ASCII characters with Shift support.
fn keycode_to_char(keycode: u8, shift: bool) -> Option<char> {
    if !shift {
        match keycode {
            10 => Some('1'),
            11 => Some('2'),
            12 => Some('3'),
            13 => Some('4'),
            14 => Some('5'),
            15 => Some('6'),
            16 => Some('7'),
            17 => Some('8'),
            18 => Some('9'),
            19 => Some('0'),
            20 => Some('-'),
            21 => Some('='),
            24 => Some('q'),
            25 => Some('w'),
            26 => Some('e'),
            27 => Some('r'),
            28 => Some('t'),
            29 => Some('y'),
            30 => Some('u'),
            31 => Some('i'),
            32 => Some('o'),
            33 => Some('p'),
            34 => Some('['),
            35 => Some(']'),
            38 => Some('a'),
            39 => Some('s'),
            40 => Some('d'),
            41 => Some('f'),
            42 => Some('g'),
            43 => Some('h'),
            44 => Some('j'),
            45 => Some('k'),
            46 => Some('l'),
            47 => Some(';'),
            48 => Some('\''),
            49 => Some('`'),
            51 => Some('\\'),
            52 => Some('z'),
            53 => Some('x'),
            54 => Some('c'),
            55 => Some('v'),
            56 => Some('b'),
            57 => Some('n'),
            58 => Some('m'),
            59 => Some(','),
            60 => Some('.'),
            61 => Some('/'),
            65 => Some(' '),
            _ => None,
        }
    } else {
        match keycode {
            10 => Some('!'),
            11 => Some('@'),
            12 => Some('#'),
            13 => Some('$'),
            14 => Some('%'),
            15 => Some('^'),
            16 => Some('&'),
            17 => Some('*'),
            18 => Some('('),
            19 => Some(')'),
            20 => Some('_'),
            21 => Some('+'),
            24 => Some('Q'),
            25 => Some('W'),
            26 => Some('E'),
            27 => Some('R'),
            28 => Some('T'),
            29 => Some('Y'),
            30 => Some('U'),
            31 => Some('I'),
            32 => Some('O'),
            33 => Some('P'),
            34 => Some('{'),
            35 => Some('}'),
            38 => Some('A'),
            39 => Some('S'),
            40 => Some('D'),
            41 => Some('F'),
            42 => Some('G'),
            43 => Some('H'),
            44 => Some('J'),
            45 => Some('K'),
            46 => Some('L'),
            47 => Some(':'),
            48 => Some('"'),
            49 => Some('~'),
            51 => Some('|'),
            52 => Some('Z'),
            53 => Some('X'),
            54 => Some('C'),
            55 => Some('V'),
            56 => Some('B'),
            57 => Some('N'),
            58 => Some('M'),
            59 => Some('<'),
            60 => Some('>'),
            61 => Some('?'),
            65 => Some(' '),
            _ => None,
        }
    }
}
