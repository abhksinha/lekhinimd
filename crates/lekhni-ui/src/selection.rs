//! Text selection interactions and clipboard copy/cut/paste integration.

use lekhni_raster::canvas::Canvas;
use lekhni_raster::damage::Rect;
#[cfg(feature = "alloc")]
use lekhni_shell::Clipboard;
#[cfg(feature = "alloc")]
use lekhni_text::buffer::PieceTable;
#[cfg(feature = "alloc")]
use lekhni_text::cursor::{Cursor, Selection};
#[cfg(feature = "alloc")]
use lekhni_text::undo::UndoManager;

/// Handles clipboard and selection actions on the text buffer.
pub struct SelectionHandler;

impl SelectionHandler {
    /// Copies the selected text to the OS clipboard.
    #[cfg(feature = "alloc")]
    pub fn copy_selection<C: Clipboard>(
        selection: &Selection,
        buffer: &PieceTable,
        clipboard: &mut C,
    ) {
        if selection.is_collapsed() {
            return;
        }
        let (start, end) = selection.byte_range();
        let len = end - start;
        let mut bytes = alloc::vec![0u8; len];
        buffer.copy_range(start, len, &mut bytes);

        if let Ok(text) = core::str::from_utf8(&bytes) {
            clipboard.set_text(text);
        }
    }

    /// Cuts the selected text to the OS clipboard and deletes it from the buffer.
    #[cfg(feature = "alloc")]
    pub fn cut_selection<C: Clipboard>(
        selection: &mut Selection,
        buffer: &mut PieceTable,
        undo_mgr: &mut UndoManager,
        clipboard: &mut C,
    ) {
        if selection.is_collapsed() {
            return;
        }
        Self::copy_selection(selection, buffer, clipboard);

        let (start, end) = selection.byte_range();
        undo_mgr.push_undo(buffer.take_snapshot(start));
        buffer.delete(start, end - start);

        let new_cursor = Cursor::new(start, 0, 0);
        *selection = Selection::collapsed(new_cursor);
    }

    /// Pastes text from the OS clipboard at the current selection/cursor position.
    #[cfg(feature = "alloc")]
    pub fn paste_clipboard<C: Clipboard>(
        selection: &mut Selection,
        buffer: &mut PieceTable,
        undo_mgr: &mut UndoManager,
        clipboard: &C,
    ) {
        let text = match clipboard.get_text() {
            Some(t) if !t.is_empty() => t,
            _ => return,
        };

        let (start, end) = selection.byte_range();
        undo_mgr.push_undo(buffer.take_snapshot(start));

        if !selection.is_collapsed() {
            buffer.delete(start, end - start);
        }

        buffer.insert(start, text.as_bytes());
        let new_offset = start + text.len();
        let new_cursor = Cursor::new(new_offset, 0, 0);
        *selection = Selection::collapsed(new_cursor);
    }

    /// Renders selection highlight rectangles on the canvas.
    pub fn draw_selection_rect(
        canvas: &mut Canvas,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        selection_color: u32,
    ) {
        canvas.fill_rect(Rect::new(x, y, width, height), selection_color);
    }
}
