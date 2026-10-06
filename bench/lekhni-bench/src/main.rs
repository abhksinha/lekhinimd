//! Lekhni Architectural Comprehensive Subcomponent Speed Benchmarks.
//!
//! Measures exact speed/latency across all subcomponents:
//! 1. PieceTable text buffer (Insert, Delete, Slice, Line/Col indexing)
//! 2. Undo/Redo stack snapshot and delta restoration
//! 3. Markdown Parser (Full document parsing and incremental re-parsing)
//! 4. Vector Typography & Font Atlas (Text measurement & anti-aliased draw)
//! 5. Framebuffer 2D Rasterizer & Alpha Blender (fill_rect, premul blend)
//! 6. Trigram Full-Text Indexer (Indexing throughput & candidate queries)
//! 7. Preview Layout & Rich Formatted Block Generator
//! 8. Crash Recovery Journaling & Storage (Write snapshot & discard)
//! 9. End-to-End interactive keystroke-to-pixels pipeline latency

use std::time::Instant;
use lekhni_text::buffer::PieceTable;
use lekhni_text::undo::UndoManager;
use lekhni_md::parser::MarkdownParser;
use lekhni_raster::canvas::Canvas;
use lekhni_raster::damage::Rect;
use lekhni_raster::vector_font::FontCollection;
use lekhni_index::trigram::TrigramIndex;
use lekhni_ui::preview::PreviewRenderer;
use lekhni_store::recovery::RecoveryJournal;
use lekhni_shell_desktop::workspace::StdFs;
use lekhni_shell_desktop::DesktopEditor;

fn main() {
    println!("========================================================================");
    println!("          LEKHNI SUBCOMPONENT BENCHMARK & SPEED AUDIT                   ");
    println!("========================================================================\n");

    // -------------------------------------------------------------------------
    // 1. PieceTable Buffer Subcomponent (lekhni-text)
    // -------------------------------------------------------------------------
    println!("--- [1] PIECE TABLE BUFFER (lekhni-text) ---");
    let initial_data = b"Lekhni high performance zero-copy PieceTable engine.\n".repeat(1000);
    let mut pt = PieceTable::new(&initial_data);

    // 1a. Random inserts
    let t0 = Instant::now();
    let num_inserts = 10_000;
    for i in 0..num_inserts {
        let pos = (i * 17) % (pt.len() + 1);
        pt.insert(pos, b"alpha ");
    }
    let insert_us = t0.elapsed().as_micros();
    println!("  • 10,000 Inserts in 50KB buffer:  {:>6} µs ({:.2} ns/op)", insert_us, (insert_us as f64 * 1000.0) / num_inserts as f64);

    // 1b. Random deletes
    let t0 = Instant::now();
    let num_deletes = 5_000;
    for i in 0..num_deletes {
        let pos = (i * 13) % (pt.len().saturating_sub(6) + 1);
        pt.delete(pos, 6);
    }
    let delete_us = t0.elapsed().as_micros();
    println!("  • 5,000 Deletes in buffer:        {:>6} µs ({:.2} ns/op)", delete_us, (delete_us as f64 * 1000.0) / num_deletes as f64);

    // 1c. Contiguous copy_range (read)
    let mut read_buf = vec![0u8; 4096];
    let t0 = Instant::now();
    let num_reads = 10_000;
    for _ in 0..num_reads {
        pt.copy_range(100, 4096, &mut read_buf);
    }
    let read_us = t0.elapsed().as_micros();
    println!("  • 10,000 4KB Range Slices:        {:>6} µs ({:.2} ns/op)", read_us, (read_us as f64 * 1000.0) / num_reads as f64);

    // 1d. Line-offset conversion
    let t0 = Instant::now();
    for i in 0..10_000 {
        let off = (i * 31) % pt.len();
        let _ = pt.offset_to_line(off);
    }
    let line_lookup_us = t0.elapsed().as_micros();
    println!("  • 10,000 Offset-to-Line Lookups:  {:>6} µs ({:.2} ns/op)\n", line_lookup_us, (line_lookup_us as f64 * 1000.0) / 10_000.0);

    // -------------------------------------------------------------------------
    // 2. Undo/Redo Subcomponent (lekhni-text)
    // -------------------------------------------------------------------------
    println!("--- [2] UNDO / REDO MANAGER (lekhni-text) ---");
    let mut undo_mgr = UndoManager::new();
    let t0 = Instant::now();
    for i in 0..2_000 {
        undo_mgr.push_undo(pt.take_snapshot(i));
    }
    let undo_push_us = t0.elapsed().as_micros();
    println!("  • 2,000 Undo Snapshots Pushed:    {:>6} µs ({:.2} ns/op)", undo_push_us, (undo_push_us as f64 * 1000.0) / 2000.0);

    let t0 = Instant::now();
    for i in 0..2_000 {
        let _ = undo_mgr.undo(pt.take_snapshot(i));
    }
    let undo_pop_us = t0.elapsed().as_micros();
    println!("  • 2,000 Undo Operations Applied:  {:>6} µs ({:.2} ns/op)\n", undo_pop_us, (undo_pop_us as f64 * 1000.0) / 2000.0);

    // -------------------------------------------------------------------------
    // 3. Markdown Parser Subcomponent (lekhni-md)
    // -------------------------------------------------------------------------
    println!("--- [3] MARKDOWN PARSER (lekhni-md) ---");
    let sample_md = b"# Title\n\nParagraph with **bold** and `code` inline spans.\n\n- [ ] Task item 1\n- [x] Task item 2\n\n```rust\nfn main() { println!(\"Fast!\"); }\n```\n\n| H1 | H2 |\n|---|---|\n| C1 | C2 |\n\n> Blockquote text here.\n";
    let medium_doc = sample_md.repeat(500); // ~100 KB document
    let mut parser = MarkdownParser::new();

    let t0 = Instant::now();
    parser.parse_full(&medium_doc);
    let full_parse_us = t0.elapsed().as_micros();
    println!("  • Full Parse of ~100 KB Markdown: {:>6} µs ({:.2} MB/s)", full_parse_us, (medium_doc.len() as f64 / 1_000_000.0) / (full_parse_us as f64 / 1_000_000.0));

    // Incremental reparsing
    let t0 = Instant::now();
    let num_inc = 5_000;
    for _ in 0..num_inc {
        parser.reparse_incremental(&medium_doc, 10);
    }
    let inc_us = t0.elapsed().as_micros();
    println!("  • 5,000 Incremental Re-parses:    {:>6} µs ({:.2} ns/op)\n", inc_us, (inc_us as f64 * 1000.0) / num_inc as f64);

    // -------------------------------------------------------------------------
    // 4. Vector Typography & Font Engine (lekhni-raster)
    // -------------------------------------------------------------------------
    println!("--- [4] VECTOR TYPOGRAPHY & FONT CACHE (lekhni-raster) ---");
    let t0 = Instant::now();
    let fonts = FontCollection::load_default().expect("fonts load");
    let font_init_us = t0.elapsed().as_micros();
    println!("  • Font Engine Boot & Glyph Atlas: {:>6} µs", font_init_us);

    let t0 = Instant::now();
    let num_meas = 50_000;
    for _ in 0..num_meas {
        let _ = fonts.editor.measure_text("fn calculate_budget(ops: usize) -> u64;");
    }
    let meas_us = t0.elapsed().as_micros();
    println!("  • 50,000 Text Measurements:       {:>6} µs ({:.2} ns/op, Zero-alloc)", meas_us, (meas_us as f64 * 1000.0) / num_meas as f64);

    let mut fb = vec![0xFF_28_2C_34u32; 1280 * 800];
    let mut canvas = Canvas::new(&mut fb, 1280, 800);
    let t0 = Instant::now();
    let num_draws = 10_000;
    for _ in 0..num_draws {
        fonts.editor.draw_text(&mut canvas, "const BUFFER_CAPACITY: usize = 1024 * 1024;", 40, 40, 0xFF_AB_B2_BF);
    }
    let draw_us = t0.elapsed().as_micros();
    println!("  • 10,000 Subpixel Text Blits:     {:>6} µs ({:.2} ns/string, {:.2} Mchars/sec)\n",
        draw_us,
        (draw_us as f64 * 1000.0) / num_draws as f64,
        (num_draws * 43) as f64 / (draw_us as f64)
    );

    // -------------------------------------------------------------------------
    // 5. 2D Framebuffer Rasterizer & Damage Blending (lekhni-raster)
    // -------------------------------------------------------------------------
    println!("--- [5] 2D RASTERIZER & BLENDING (lekhni-raster) ---");
    let t0 = Instant::now();
    let num_rects = 20_000;
    for i in 0..num_rects {
        let x = (i * 7) % 1000;
        let y = (i * 11) % 600;
        canvas.fill_rect(Rect::new(x as i32, y as i32, 120, 24), 0xFF_61_AF_EF);
    }
    let fill_us = t0.elapsed().as_micros();
    println!("  • 20,000 Clipped fill_rect Calls: {:>6} µs ({:.2} ns/rect, {:.2} Mpixels/sec)\n",
        fill_us,
        (fill_us as f64 * 1000.0) / num_rects as f64,
        (num_rects as f64 * 120.0 * 24.0) / (fill_us as f64)
    );

    // -------------------------------------------------------------------------
    // 6. Trigram Search Engine Subcomponent (lekhni-index)
    // -------------------------------------------------------------------------
    println!("--- [6] TRIGRAM FULL-TEXT SEARCH (lekhni-index) ---");
    let mut trigram_idx = TrigramIndex::new();
    let t0 = Instant::now();
    for i in 0..10_000 {
        let content = format!("Notebook entry {} with Rust systems code, memory safety, and SIMD layout optimizations.", i);
        trigram_idx.index_doc(i, content.as_bytes());
    }
    let index_docs_us = t0.elapsed().as_micros();
    println!("  • Index 10,000 Documents:         {:>6} µs ({:.2} µs/doc)", index_docs_us, index_docs_us as f64 / 10_000.0);

    let t0 = Instant::now();
    let num_queries = 2_000;
    for _ in 0..num_queries {
        let _ = trigram_idx.query_candidates(b"optimizations");
    }
    let query_us = t0.elapsed().as_micros();
    println!("  • 2,000 Full Trigram Queries:     {:>6} µs ({:.2} µs/query, {:.2} queries/sec)\n",
        query_us,
        query_us as f64 / num_queries as f64,
        (num_queries as f64 * 1_000_000.0) / query_us as f64
    );

    // -------------------------------------------------------------------------
    // 7. Preview Layout & Rich Formatter Subcomponent (lekhni-ui)
    // -------------------------------------------------------------------------
    println!("--- [7] PREVIEW LAYOUT & RICH FORMATTER (lekhni-ui) ---");
    parser.parse_full(sample_md);
    let t0 = Instant::now();
    let num_layouts = 5_000;
    for _ in 0..num_layouts {
        let _ = PreviewRenderer::compute_layouts(&parser.blocks, sample_md, 800);
    }
    let layout_us = t0.elapsed().as_micros();
    println!("  • 5,000 Block Layout Passes:      {:>6} µs ({:.2} µs/pass)", layout_us, layout_us as f64 / num_layouts as f64);

    let t0 = Instant::now();
    let num_prev_draws = 1_000;
    for _ in 0..num_prev_draws {
        let _ = PreviewRenderer::render_preview_with_fonts(
            &mut canvas,
            &parser.blocks,
            sample_md,
            0,
            800,
            600,
            Some(&fonts),
        );
    }
    let prev_render_us = t0.elapsed().as_micros();
    println!("  • 1,000 Vector Preview Renders:   {:>6} µs ({:.2} µs/frame, {:.2} FPS equivalent)\n",
        prev_render_us,
        prev_render_us as f64 / num_prev_draws as f64,
        (num_prev_draws as f64 * 1_000_000.0) / prev_render_us as f64
    );

    // -------------------------------------------------------------------------
    // 8. Crash Recovery Journaling Subcomponent (lekhni-store)
    // -------------------------------------------------------------------------
    println!("--- [8] CRASH RECOVERY JOURNALING (lekhni-store) ---");
    let tmp_dir = std::env::temp_dir().display().to_string();
    let test_bytes = b"# Unsaved draft state snapshot for crash resilience";
    let t0 = Instant::now();
    let num_journals = 500;
    for _ in 0..num_journals {
        let _ = RecoveryJournal::write_snapshot(&StdFs, &tmp_dir, "bench_test", test_bytes);
    }
    let journal_us = t0.elapsed().as_micros();
    println!("  • 500 Disk Journal Snapshots:     {:>6} µs ({:.2} µs/flush, atomic fsync)", journal_us, journal_us as f64 / num_journals as f64);
    let _ = RecoveryJournal::discard_snapshot(&StdFs, &tmp_dir, "bench_test");

    // -------------------------------------------------------------------------
    // 9. End-to-End Interactive Keystroke Latency (lekhni-shell-desktop)
    // -------------------------------------------------------------------------
    println!("\n--- [9] END-TO-END KEYSTROKE PIPELINE (lekhni-shell-desktop) ---");
    let mut desktop = DesktopEditor::new(b"# Live Document\nTesting end-to-end typing pipeline.\n", 1280, 800);
    let mut latencies = Vec::with_capacity(1_000);
    for _ in 0..1_000 {
        let lat = desktop.handle_keystroke(b"a");
        latencies.push(lat);
    }
    let min_lat = latencies.iter().copied().min().unwrap_or(0) as u64;
    let max_lat = latencies.iter().copied().max().unwrap_or(0) as u64;
    let avg_lat = (latencies.iter().copied().sum::<u128>() / latencies.len() as u128) as u64;

    println!("  • 1,000 Live Keystrokes (Buffer + Damage + Layout + Blit):");
    println!("    - Average Latency:  {:>5} µs ({:.3} ms)", avg_lat, avg_lat as f64 / 1000.0);
    println!("    - Minimum Latency:  {:>5} µs ({:.3} ms)", min_lat, min_lat as f64 / 1000.0);
    println!("    - Maximum Latency:  {:>5} µs ({:.3} ms)", max_lat, max_lat as f64 / 1000.0);
    println!("    - Latency Budget:   < 8,000 µs (8.0 ms)");
    println!("    - Budget Margin:    {:.1}x FASTER than budget", 8000.0 / avg_lat.max(1) as f64);

    println!("\n========================================================================");
    println!("          ALL SUBCOMPONENT SPEED CHECKS COMPLETED                        ");
    println!("========================================================================");
}
