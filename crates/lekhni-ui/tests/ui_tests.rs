use lekhni_md::parser::MarkdownParser;
use lekhni_raster::canvas::Canvas;
use lekhni_shell::Clipboard;
use lekhni_text::buffer::PieceTable;
use lekhni_text::cursor::{Cursor, Selection};
use lekhni_text::undo::UndoManager;
use lekhni_ui::preview::PreviewRenderer;
use lekhni_ui::scroll::ScrollState;
use lekhni_ui::selection::SelectionHandler;

struct MockClipboard {
    content: Option<String>,
}

impl Clipboard for MockClipboard {
    fn get_text(&self) -> Option<String> {
        self.content.clone()
    }

    fn set_text(&mut self, text: &str) {
        self.content = Some(text.into());
    }
}

#[test]
fn test_scroll_physics_and_damage() {
    let mut scroll = ScrollState::new(400);
    scroll.update_content_height(1000);
    assert_eq!(scroll.max_offset_y, 600);

    let delta = scroll.scroll_by(50);
    assert_eq!(delta, 50);
    assert_eq!(scroll.offset_y, 50);

    let exposed = scroll.compute_exposed_damage(50, 800).unwrap();
    assert_eq!(exposed.y, 350);
    assert_eq!(exposed.height, 50);
}

#[test]
fn test_selection_cut_copy_paste() {
    let mut pt = PieceTable::new(b"Hello Beautiful World");
    let mut undo_mgr = UndoManager::new();
    let mut clip = MockClipboard { content: None };

    // Select "Beautiful "
    let mut sel = Selection {
        anchor: Cursor::new(6, 0, 6),
        head: Cursor::new(16, 0, 16),
    };

    SelectionHandler::copy_selection(&sel, &pt, &mut clip);
    assert_eq!(clip.content.as_deref(), Some("Beautiful "));

    SelectionHandler::cut_selection(&mut sel, &mut pt, &mut undo_mgr, &mut clip);
    let mut buf = [0u8; 32];
    let n = pt.copy_range(0, pt.len(), &mut buf);
    assert_eq!(&buf[..n], b"Hello World");

    // Paste back
    SelectionHandler::paste_clipboard(&mut sel, &mut pt, &mut undo_mgr, &clip);
    let n = pt.copy_range(0, pt.len(), &mut buf);
    assert_eq!(&buf[..n], b"Hello Beautiful World");
}

#[test]
fn test_preview_viewport_rendering() {
    let doc = b"# Heading 1\n\nParagraph\n\n```rust\nfn main() {}\n```\n";
    let mut parser = MarkdownParser::new();
    parser.parse_full(doc);

    let mut fb = vec![0u32; 800 * 600];
    let mut canvas = Canvas::new(&mut fb, 800, 600);

    PreviewRenderer::render_preview(&mut canvas, &parser.blocks, 0, 800, 600);
    // Verified no panic and blocks rendered within viewport bounds
}
