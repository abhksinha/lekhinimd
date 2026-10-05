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
