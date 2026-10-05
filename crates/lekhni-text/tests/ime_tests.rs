use lekhni_text::buffer::PieceTable;
use lekhni_text::cursor::Cursor;
use lekhni_text::ime::{ImeEvent, ImeManager};
use lekhni_text::shaper::{requires_complex_shaping, Shaper, SimpleShaper};

#[test]
fn test_ime_lifecycle_devanagari_hindi() {
    let mut pt = PieceTable::new(b"");
    let mut cursor = Cursor::default();
    let mut ime = ImeManager::new();

    // 1. Enable IME
    ime.handle_event(ImeEvent::Enabled, &mut pt, &mut cursor);
    assert!(ime.is_enabled);

    // 2. Typing phonetic "lekh" in preedit
    ime.handle_event(
        ImeEvent::Preedit("lekh".into(), Some((4, 4))),
        &mut pt,
        &mut cursor,
    );
    assert_eq!(ime.preedit_text(), Some("lekh"));
    assert_eq!(pt.len(), 0); // No text committed yet

    // 3. User selects Hindi Devanagari candidate "लेखनी" (Lekhni)
    let hindi_word = "लेखनी";
    ime.handle_event(
        ImeEvent::Commit(hindi_word.into()),
        &mut pt,
        &mut cursor,
    );

    assert_eq!(ime.preedit_text(), None);
    assert_eq!(pt.len(), hindi_word.len());

    let mut out = vec![0u8; pt.len()];
    pt.copy_range(0, pt.len(), &mut out);
    assert_eq!(std::str::from_utf8(&out).unwrap(), hindi_word);
}

#[test]
fn test_ime_cjk_commit() {
    let mut pt = PieceTable::new(b"");
    let mut cursor = Cursor::default();
    let mut ime = ImeManager::new();

    ime.handle_event(ImeEvent::Enabled, &mut pt, &mut cursor);

    // Chinese Pinyin "nihao" -> "你好"
    let cjk_text = "你好";
    ime.handle_event(ImeEvent::Commit(cjk_text.into()), &mut pt, &mut cursor);
    assert_eq!(pt.len(), cjk_text.len());
}

#[test]
fn test_complex_script_detection() {
    assert!(requires_complex_shaping("नमस्ते")); // Devanagari
    assert!(requires_complex_shaping("مرحبا")); // Arabic
    assert!(!requires_complex_shaping("Hello world 123")); // Plain Latin
}

#[test]
fn test_shaper_glyph_run() {
    let shaper = SimpleShaper;
    let glyphs = shaper.shape_line("लेखनी", 14.0);
    assert_eq!(glyphs.len(), "लेखनी".chars().count());
}
