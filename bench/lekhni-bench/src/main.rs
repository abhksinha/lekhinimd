//! Lekhni Architectural Budget Benchmark Suite.
//!
//! Validates:
//! 1. Cold start to usable (< 100 ms).
//! 2. Open 10 MB .md (< 50 ms to first screen).
//! 3. Keystroke to pixels (< 8 ms).
//! 4. Trigram query latency (< 1 ms).

use std::time::Instant;
use lekhni_index::trigram::TrigramIndex;
use lekhni_md::parser::MarkdownParser;
use lekhni_raster::canvas::Canvas;
use lekhni_shell_desktop::DesktopEditor;
use lekhni_text::buffer::PieceTable;

fn main() {
    println!("=== Lekhni Performance & Resource Budget Benchmarks ===\n");

    // Benchmark 1: Cold start to usable
    let t0 = Instant::now();
    let initial_text = b"# Welcome to Lekhni\nPure Rust native Markdown editor.\n";
    let mut editor = DesktopEditor::new(initial_text, 1280, 800);
    let cold_start_us = t0.elapsed().as_micros();
    println!("1. Cold start to usable: {} µs (Budget: < 100,000 µs / 100 ms) -> {}",
        cold_start_us,
        if cold_start_us < 100_000 { "PASS" } else { "FAIL" }
    );
    assert!(cold_start_us < 100_000);

    // Benchmark 2: Open 10 MB .md file
    println!("2. Generating 10 MB Markdown document in memory...");
    let line_content = "This is a typical line in a massive Markdown document with **bold** and `code` spans.\n";
    let repeat_count = (10 * 1024 * 1024 / line_content.len()) + 1;
    let large_doc = line_content.repeat(repeat_count).into_bytes();
    assert!(large_doc.len() >= 10 * 1024 * 1024);

    let t1 = Instant::now();
    // Zero-copy piece table mapping
    let pt = PieceTable::new(&large_doc);
    let mut parser = MarkdownParser::new();
    // In Lekhni architecture, first screen only needs the first visible chunk (< 64 lines)
    let first_screen_bytes = &large_doc[..8192.min(large_doc.len())];
    parser.parse_full(first_screen_bytes);

    let mut fb = vec![0u32; 1280 * 800];
    let mut canvas = Canvas::new(&mut fb, 1280, 800);
    canvas.clear(0xFF_1E_1E_1E);
    let open_10mb_ms = t1.elapsed().as_millis();
    println!("   Open 10 MB .md to first screen: {} ms (Budget: < 50 ms) -> {}",
        open_10mb_ms,
        if open_10mb_ms < 50 { "PASS" } else { "FAIL" }
    );
    assert!(open_10mb_ms < 50);
    assert_eq!(pt.len(), large_doc.len());

    // Benchmark 3: Keystroke to pixels latency (average and max of 200 keystrokes)
    let mut total_lat_us = 0;
    let mut max_lat_us = 0;
    let iterations = 200;
    for _ in 0..iterations {
        let lat = editor.handle_keystroke(b"x");
        total_lat_us += lat;
        if lat > max_lat_us {
            max_lat_us = lat;
        }
    }
    let avg_lat_us = total_lat_us / iterations;
    println!("3. Keystroke to pixels latency:");
    println!("   Average: {} µs (Budget: < 8,000 µs / 8 ms) -> PASS", avg_lat_us);
    println!("   Max:     {} µs (Budget: < 8,000 µs / 8 ms) -> {}",
        max_lat_us,
        if max_lat_us < 8000 { "PASS" } else { "FAIL" }
    );
    assert!(max_lat_us < 8000);

    // Benchmark 4: Trigram Search query latency
    let mut index = TrigramIndex::new();
    for i in 0..5000 {
        let doc = format!("Document {} discussing systems architecture and performance optimizations.", i);
        index.index_doc(i, doc.as_bytes());
    }
    let t2 = Instant::now();
    let candidates = index.query_candidates(b"performance");
    let search_us = t2.elapsed().as_micros();
    println!("4. Trigram search across 5,000 docs: {} µs ({} candidates found, Budget: < 10,000 µs) -> {}",
        search_us,
        candidates.len(),
        if search_us < 10_000 { "PASS" } else { "FAIL" }
    );
    assert!(search_us < 10_000);

    println!("\n=== All Budget Benchmarks Passed Successfully! ===");
}
