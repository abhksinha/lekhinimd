use lekhni_shell_desktop::workspace::StdFs;
use lekhni_store::recovery::RecoveryJournal;
use lekhni_text::buffer::PieceTable;
use lekhni_text::cursor::{Cursor, Selection};
use lekhni_text::undo::UndoManager;

#[test]
fn test_unicode_char_boundary_navigation() {
    // "लेखनी" has multi-byte UTF-8 sequences
    let text = "लेखनी Markdown".as_bytes();
    let buffer = PieceTable::new(text);

    // Verify boundaries don't land in the middle of UTF-8 continuation bytes
    let mut offset = 0;
    while offset < buffer.len() {
        let b = buffer.byte_at(offset).unwrap();
        // Leading byte must not be continuation byte (0x80..=0xBF)
        assert!((b & 0xC0) != 0x80, "Offset {} landed on continuation byte", offset);

        // Advance to next char boundary
        let mut next = offset + 1;
        while next < buffer.len() {
            if let Some(nb) = buffer.byte_at(next) {
                if (nb & 0xC0) != 0x80 {
                    break;
                }
            }
            next += 1;
        }
        offset = next;
    }
}

#[test]
fn test_selection_range_and_collapse() {
    let anchor = Cursor::new(5, 0, 5);
    let head = Cursor::new(15, 0, 15);
    let sel = Selection { anchor, head };

    assert!(!sel.is_collapsed());
    assert_eq!(sel.byte_range(), (5, 15));

    let reversed_sel = Selection { anchor: head, head: anchor };
    assert_eq!(reversed_sel.byte_range(), (5, 15));

    let collapsed = Selection::collapsed(anchor);
    assert!(collapsed.is_collapsed());
    assert_eq!(collapsed.byte_range(), (5, 5));
}

#[test]
fn test_undo_redo_stack_flow() {
    let mut buffer = PieceTable::new(b"Hello");
    let mut undo_mgr = UndoManager::new();

    undo_mgr.push_undo(buffer.take_snapshot(5));
    buffer.insert(5, b" World");
    assert_eq!(buffer.len(), 11);

    // Undo
    let prev = undo_mgr.undo(buffer.take_snapshot(11)).expect("Undo snapshot exists");
    buffer.restore_snapshot(prev);
    assert_eq!(buffer.len(), 5);

    // Redo
    let next = undo_mgr.redo(buffer.take_snapshot(5)).expect("Redo snapshot exists");
    buffer.restore_snapshot(next);
    assert_eq!(buffer.len(), 11);
}

#[test]
fn test_recovery_journal_atomic_lifecycle() {
    let temp_dir = std::env::temp_dir().join(format!("lekhni_rec_test_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&temp_dir);
    let root = temp_dir.display().to_string();
    let page = "test_note.md";
    let dirty_data = b"# Unsaved Session Content\n\nMust recover safely!\n";

    // 1. Write snapshot
    let write_res = RecoveryJournal::write_snapshot(&StdFs, &root, page, dirty_data);
    assert!(write_res.is_ok());

    let rec_path = RecoveryJournal::journal_path(&root, page);
    let read_data = std::fs::read(&rec_path).expect("Recovery file exists");
    assert_eq!(read_data, dirty_data);

    // 2. Discard snapshot on clean save
    let discard_res = RecoveryJournal::discard_snapshot(&StdFs, &root, page);
    assert!(discard_res.is_ok());
    assert!(!std::path::Path::new(&rec_path).exists());

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_workspace_mode_layout_invariants() {
    use lekhni_layout::{compute_workspace_layout_with_mode, WorkspaceMode};

    let width = 1280;
    let height = 800;
    let sidebar_w = 200;
    let page_list_w = 240;
    let splitter_w = 4;
    let menu_bar_h = 32;
    let status_bar_h = 24;

    // 1. Split mode
    let split_layout = compute_workspace_layout_with_mode(
        width, height, sidebar_w, page_list_w, splitter_w, WorkspaceMode::Split, menu_bar_h, status_bar_h
    );
    assert_eq!(split_layout.menu_bar_rect.height, 32);
    assert_eq!(split_layout.status_bar_rect.height, 24);
    assert!(split_layout.editor_rect.width > 0);
    assert!(split_layout.preview_rect.width > 0);
    assert_eq!(split_layout.editor_rect.y, 32);
    assert_eq!(split_layout.preview_rect.y, 32);
    assert_eq!(split_layout.editor_rect.height, height - 32 - 24);
    assert_eq!(split_layout.preview_rect.height, height - 32 - 24);

    // 2. EditorOnly mode
    let editor_layout = compute_workspace_layout_with_mode(
        width, height, sidebar_w, page_list_w, splitter_w, WorkspaceMode::EditorOnly, menu_bar_h, status_bar_h
    );
    assert!(editor_layout.editor_rect.width > split_layout.editor_rect.width);
    assert_eq!(editor_layout.preview_rect.width, 0);

    // 3. PreviewOnly mode
    let preview_layout = compute_workspace_layout_with_mode(
        width, height, sidebar_w, page_list_w, splitter_w, WorkspaceMode::PreviewOnly, menu_bar_h, status_bar_h
    );
    assert_eq!(preview_layout.editor_rect.width, 0);
    assert!(preview_layout.preview_rect.width > split_layout.preview_rect.width);
}
