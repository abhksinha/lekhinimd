//! Lekhni Desktop Interactive Native GUI Runner.
//!
//! Pure Rust native desktop frontend. No Webview, no Electron, no Tauri.
//! Direct zero-copy presentation to Linux display surface with zero idle CPU.

use lekhni_layout::compute_workspace_layout;
use lekhni_md::parser::MarkdownParser;
use lekhni_raster::canvas::Canvas;
use lekhni_shell_desktop::x11_presenter::X11Window;
use lekhni_text::buffer::PieceTable;
use lekhni_ui::preview::PreviewRenderer;
use lekhni_ui::theme::Theme;

fn main() {
    println!("=== Launching Lekhni Pure Rust Native Editor ===");
    let width = 1100u16;
    let height = 700u16;

    let theme = Theme::dark();
    let mut fb = vec![theme.bg_workspace; (width as usize) * (height as usize)];

    let initial_doc = b"# \xe0\xa4\xb2\xe0\xa5\x87\xe0\xa4\x96\xe0\xa4\xa8\xe0\xa5\x80 (Lekhni)\n\nWelcome to **Lekhni** \xe2\x80\x94 the world-class pure Rust Markdown notebook.\n\n```rust\nfn main() {\n    println!(\"Zero heap per keystroke!\\n\");\n}\n```\n\n> 1980s discipline: small, fixed, measured.\n\n- [x] Pure Rust no_std core\n- [x] Sub-8ms keystroke latency\n- [x] Real-time live preview\n";
    let mut buffer = PieceTable::new(initial_doc);
    let mut parser = MarkdownParser::new();

    let redraw = |fb: &mut [u32], buffer: &PieceTable, parser: &mut MarkdownParser| {
        let mut canvas = Canvas::new(fb, width as u32, height as u32);
        canvas.clear(theme.bg_workspace);

        // 1. Compute 3-pane workspace geometry
        let layout = compute_workspace_layout(width as i32, height as i32, 180, 220, 2, true);

        // 2. Draw Panes
        canvas.fill_rect(layout.sidebar_rect, theme.bg_sidebar);
        canvas.fill_rect(layout.splitter_1_rect, theme.border_color);
        canvas.fill_rect(layout.page_list_rect, theme.bg_page_list);
        canvas.fill_rect(layout.splitter_2_rect, theme.border_color);
        canvas.fill_rect(layout.editor_rect, theme.bg_editor);
        canvas.fill_rect(layout.preview_rect, theme.bg_preview);

        // 3. Re-parse and render live preview
        let mut doc_bytes = vec![0u8; buffer.len()];
        buffer.copy_range(0, buffer.len(), &mut doc_bytes);
        parser.parse_full(&doc_bytes);

        let mut preview_canvas = Canvas::new(fb, width as u32, height as u32);
        preview_canvas.clip = layout.preview_rect;
        PreviewRenderer::render_preview(
            &mut preview_canvas,
            &parser.blocks,
            0,
            layout.preview_rect.width,
            layout.preview_rect.height,
        );
    };

    redraw(&mut fb, &buffer, &mut parser);

    // 4. Open native X11 window
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
    let _ = win.present_framebuffer(&fb, width, height);
    println!("Lekhni native desktop window is live!");
    println!("Interactive session active: type to edit document, press Escape to exit.");

    // 5. Interactive Event Loop (blocking on socket: 0% idle CPU)
    loop {
        match win.wait_event() {
            Ok(event) => {
                let event_type = event[0] & 0x7F;
                match event_type {
                    // Expose event (opcode 12): re-present window framebuffer
                    12 => {
                        let _ = win.present_framebuffer(&fb, width, height);
                    }
                    // KeyPress event (opcode 2)
                    2 => {
                        let keycode = event[1];
                        if keycode == 9 {
                            // Escape: exit cleanly
                            println!("\nEscape pressed. Exiting Lekhni.");
                            break;
                        } else if keycode == 22 {
                            // Backspace
                            if !buffer.is_empty() {
                                buffer.delete(buffer.len() - 1, 1);
                                redraw(&mut fb, &buffer, &mut parser);
                                let _ = win.present_framebuffer(&fb, width, height);
                            }
                        } else if let Some(ascii_char) = keycode_to_ascii(keycode) {
                            buffer.insert(buffer.len(), &[ascii_char]);
                            redraw(&mut fb, &buffer, &mut parser);
                            let _ = win.present_framebuffer(&fb, width, height);
                        }
                    }
                    _ => {}
                }
            }
            Err(e) => {
                eprintln!("Connection closed: {}", e);
                break;
            }
        }
    }
}

/// Translates Linux evdev/X11 keycodes to ASCII characters.
fn keycode_to_ascii(keycode: u8) -> Option<u8> {
    match keycode {
        10 => Some(b'1'),
        11 => Some(b'2'),
        12 => Some(b'3'),
        13 => Some(b'4'),
        14 => Some(b'5'),
        15 => Some(b'6'),
        16 => Some(b'7'),
        17 => Some(b'8'),
        18 => Some(b'9'),
        19 => Some(b'0'),
        20 => Some(b'-'),
        21 => Some(b'='),
        24 => Some(b'q'),
        25 => Some(b'w'),
        26 => Some(b'e'),
        27 => Some(b'r'),
        28 => Some(b't'),
        29 => Some(b'y'),
        30 => Some(b'u'),
        31 => Some(b'i'),
        32 => Some(b'o'),
        33 => Some(b'p'),
        36 => Some(b'\n'), // Enter / Return
        38 => Some(b'a'),
        39 => Some(b's'),
        40 => Some(b'd'),
        41 => Some(b'f'),
        42 => Some(b'g'),
        43 => Some(b'h'),
        44 => Some(b'j'),
        45 => Some(b'k'),
        46 => Some(b'l'),
        52 => Some(b'z'),
        53 => Some(b'x'),
        54 => Some(b'c'),
        55 => Some(b'v'),
        56 => Some(b'b'),
        57 => Some(b'n'),
        58 => Some(b'm'),
        65 => Some(b' '), // Space
        _ => None,
    }
}
