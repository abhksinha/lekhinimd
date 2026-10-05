use lekhni_shell_desktop::DesktopEditor;

#[test]
fn test_keystroke_latency_under_budget() {
    let initial = b"# Lekhni Latency Test\nLine 1\nLine 2\nLine 3\nLine 4\nLine 5\n";
    let mut editor = DesktopEditor::new(initial, 800, 600);

    // Warm-up keystroke
    editor.handle_keystroke(b"a");

    // Execute 100 consecutive keystrokes
    let mut max_latency_us = 0;
    for _ in 0..100 {
        let lat = editor.handle_keystroke(b"x");
        if lat > max_latency_us {
            max_latency_us = lat;
        }
    }

    // Budget: <= 8000 microseconds (8 ms)
    // In our optimized zero-copy engine, it should be well under 1000 microseconds (1 ms)!
    assert!(
        max_latency_us < 8000,
        "Keystroke latency exceeded 8ms budget: {} us",
        max_latency_us
    );
}
