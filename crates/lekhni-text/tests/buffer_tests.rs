use lekhni_text::buffer::PieceTable;

#[test]
fn test_piece_table_initial() {
    let initial = b"Hello, world!\nLine 2\n";
    let pt = PieceTable::new(initial);
    assert_eq!(pt.len(), initial.len());
    assert_eq!(pt.line_breaks(), 2);

    let mut buf = [0u8; 32];
    let n = pt.copy_range(0, pt.len(), &mut buf);
    assert_eq!(&buf[..n], initial);
}

#[test]
fn test_piece_coalescing_on_append() {
    let mut pt = PieceTable::new(b"");
    pt.insert(0, b"a");
    pt.insert(1, b"b");
    pt.insert(2, b"c");

    assert_eq!(pt.len(), 3);
    let mut buf = [0u8; 10];
    let n = pt.copy_range(0, 3, &mut buf);
    assert_eq!(&buf[..n], b"abc");
}

#[test]
fn test_piece_table_insert_middle_and_delete() {
    let initial = b"First Last";
    let mut pt = PieceTable::new(initial);

    // Insert " Middle" between First and Last
    pt.insert(5, b" Middle");
    let mut buf = [0u8; 32];
    let n = pt.copy_range(0, pt.len(), &mut buf);
    assert_eq!(&buf[..n], b"First Middle Last");

    // Delete " Middle"
    pt.delete(5, 7);
    let n = pt.copy_range(0, pt.len(), &mut buf);
    assert_eq!(&buf[..n], b"First Last");
}

#[test]
fn test_line_index_navigation() {
    let text = b"Line 1\nLine 2\nLine 3\n";
    let pt = PieceTable::new(text);

    assert_eq!(pt.line_to_offset(0), 0);
    assert_eq!(pt.line_to_offset(1), 7);
    assert_eq!(pt.line_to_offset(2), 14);

    let (line, start) = pt.offset_to_line(8); // inside "Line 2"
    assert_eq!(line, 1);
    assert_eq!(start, 7);
}

#[test]
fn test_undo_redo() {
    use lekhni_text::undo::UndoManager;

    let mut pt = PieceTable::new(b"Hello");
    let mut undo_mgr = UndoManager::new();

    // Snapshot before edit
    undo_mgr.push_undo(pt.take_snapshot(5));
    pt.insert(5, b" World");

    let mut buf = [0u8; 16];
    let n = pt.copy_range(0, pt.len(), &mut buf);
    assert_eq!(&buf[..n], b"Hello World");

    // Perform Undo
    let snap_current = pt.take_snapshot(11);
    let prev = undo_mgr.undo(snap_current).expect("undo snapshot available");
    pt.restore_snapshot(prev);

    let n = pt.copy_range(0, pt.len(), &mut buf);
    assert_eq!(&buf[..n], b"Hello");

    // Perform Redo
    let snap_current = pt.take_snapshot(5);
    let next = undo_mgr.redo(snap_current).expect("redo snapshot available");
    pt.restore_snapshot(next);

    let n = pt.copy_range(0, pt.len(), &mut buf);
    assert_eq!(&buf[..n], b"Hello World");
}

