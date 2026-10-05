//! Lekhni Desktop Interactive Native GUI Runner.
//!
//! Pure Rust native desktop frontend. No Webview, no Electron, no Tauri.
//! Direct zero-copy presentation to Linux display surface.

use lekhni_layout::compute_workspace_layout;
use lekhni_md::parser::MarkdownParser;
use lekhni_raster::canvas::Canvas;
use lekhni_shell_desktop::x11_presenter::X11Window;
use lekhni_ui::preview::PreviewRenderer;
use lekhni_ui::theme::Theme;

fn main() {
    println!("=== Launching Lekhni Pure Rust Native Editor ===");
    let width = 1100u16;
    let height = 700u16;

    let theme = Theme::dark();
    let mut fb = vec![theme.bg_workspace; (width as usize) * (height as usize)];
    let mut canvas = Canvas::new(&mut fb, width as u32, height as u32);

    // 1. Compute 3-pane workspace geometry
    let layout = compute_workspace_layout(width as i32, height as i32, 180, 220, 2, true);

    // 2. Draw Sidebar
    canvas.fill_rect(layout.sidebar_rect, theme.bg_sidebar);
    // Draw Splitter 1
    canvas.fill_rect(layout.splitter_1_rect, theme.border_color);
    // Draw Page List
    canvas.fill_rect(layout.page_list_rect, theme.bg_page_list);
    // Draw Splitter 2
    canvas.fill_rect(layout.splitter_2_rect, theme.border_color);
    // Draw Editor pane
    canvas.fill_rect(layout.editor_rect, theme.bg_editor);
    // Draw Preview pane
    canvas.fill_rect(layout.preview_rect, theme.bg_preview);

    // 3. Render Sample Markdown Document in Editor & Live Preview
    let sample_doc = b"# \xe0\xa4\xb2\xe0\xa5\x87\xe0\xa4\x96\xe0\xa4\xa8\xe0\xa5\x80 (Lekhni)\n\nWelcome to **Lekhni** \xe2\x80\x94 the world-class pure Rust Markdown notebook.\n\n```rust\nfn main() {\n    println!(\"Zero heap per keystroke!\\n\");\n}\n```\n\n> 1980s discipline: small, fixed, measured.\n\n- [x] Pure Rust no_std core\n- [x] Sub-8ms keystroke latency\n- [x] Real-time live preview\n";
    let mut parser = MarkdownParser::new();
    parser.parse_full(sample_doc);

    // Render Preview into preview rect
    let preview_clip = layout.preview_rect;
    let mut preview_canvas = Canvas::new(&mut fb, width as u32, height as u32);
    preview_canvas.clip = preview_clip;
    PreviewRenderer::render_preview(&mut preview_canvas, &parser.blocks, 0, layout.preview_rect.width, layout.preview_rect.height);

    // 4. Open native X11 window
    println!("Connecting to display and opening native window...");
    match X11Window::open("Lekhni Markdown Editor (Native Pure Rust)", width, height) {
        Ok(mut win) => {
            println!("Window mapped successfully! Window ID: 0x{:x}", win.window_id);
            if let Err(e) = win.present_framebuffer(&fb, width, height) {
                eprintln!("Presentation error: {}", e);
            }
            println!("Presented initial frame in pure Rust to X11 surface.");
            println!("Lekhni native desktop window is live!");
        }
        Err(e) => {
            eprintln!("Native display presentation notice: {}", e);
            eprintln!("Desktop headless and CLI modes remain fully functional.");
        }
    }
}
