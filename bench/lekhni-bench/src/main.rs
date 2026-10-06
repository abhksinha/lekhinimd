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

use std::hint::black_box;
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

    // 1a. Sequential typing inserts (coalescing path)
    let t0 = Instant::now();
    let num_typing = 10_000;
    for _ in 0..num_typing {
        pt.insert(pt.len(), black_box(b"a"));
    }
    let typing_us = t0.elapsed().as_micros();
    println!("  • 10,000 Typing Inserts (Coalesced):  {:>6} µs ({:.2} ns/op)", typing_us, (typing_us as f64 * 1000.0) / num_typing as f64);

    // 1b. Random inserts (splitting path)
    let t0 = Instant::now();
    let num_inserts = 10_000;
    for i in 0..num_inserts {
        let pos = (i * 17) % (pt.len() + 1);
        pt.insert(pos, black_box(b"alpha "));
    }
    let insert_us = t0.elapsed().as_micros();
    println!("  • 10,000 Random Inserts in Buffer:    {:>6} µs ({:.2} ns/op)", insert_us, (insert_us as f64 * 1000.0) / num_inserts as f64);

    // 1c. Random deletes
    let t0 = Instant::now();
    let num_deletes = 5_000;
    for i in 0..num_deletes {
        let pos = (i * 13) % (pt.len().saturating_sub(6) + 1);
        pt.delete(pos, black_box(6));
    }
    let delete_us = t0.elapsed().as_micros();
    println!("  • 5,000 Deletes in Buffer:            {:>6} µs ({:.2} ns/op)", delete_us, (delete_us as f64 * 1000.0) / num_deletes as f64);

    // 1d. Contiguous copy_range (read)
    let mut read_buf = vec![0u8; 4096];
    let t0 = Instant::now();
    let num_reads = 10_000;
    for _ in 0..num_reads {
        pt.copy_range(black_box(100), black_box(4096), black_box(&mut read_buf));
    }
    let read_us = t0.elapsed().as_micros();
    println!("  • 10,000 4KB Range Slices:            {:>6} µs ({:.2} ns/op)", read_us, (read_us as f64 * 1000.0) / num_reads as f64);

    // 1e. Line-offset conversion
    let t0 = Instant::now();
    for i in 0..10_000 {
        let off = (i * 31) % pt.len();
        let res = pt.offset_to_line(black_box(off));
        black_box(res);
    }
    let line_lookup_us = t0.elapsed().as_micros();
    println!("  • 10,000 Offset-to-Line Lookups:      {:>6} µs ({:.2} ns/op)\n", line_lookup_us, (line_lookup_us as f64 * 1000.0) / 10_000.0);

    // -------------------------------------------------------------------------
    // 2. Undo/Redo Subcomponent (lekhni-text)
    // -------------------------------------------------------------------------
    println!("--- [2] UNDO / REDO MANAGER (lekhni-text) ---");
    let mut undo_mgr = UndoManager::new();
    let t0 = Instant::now();
    for i in 0..2_000 {
        undo_mgr.push_undo(black_box(pt.take_snapshot(i)));
    }
    let undo_push_us = t0.elapsed().as_micros();
    println!("  • 2,000 Undo Snapshots Pushed:        {:>6} µs ({:.2} ns/op, Arc O(1))", undo_push_us, (undo_push_us as f64 * 1000.0) / 2000.0);

    let t0 = Instant::now();
    for i in 0..2_000 {
        let res = undo_mgr.undo(black_box(pt.take_snapshot(i)));
        black_box(res);
    }
    let undo_pop_us = t0.elapsed().as_micros();
    println!("  • 2,000 Undo Operations Applied:      {:>6} µs ({:.2} ns/op)\n", undo_pop_us, (undo_pop_us as f64 * 1000.0) / 2000.0);

    // -------------------------------------------------------------------------
    // 3. Markdown Parser Subcomponent (lekhni-md)
    // -------------------------------------------------------------------------
    println!("--- [3] MARKDOWN PARSER (lekhni-md) ---");
    let sample_md = b"# Title\n\nParagraph with **bold** and `code` inline spans.\n\n- [ ] Task item 1\n- [x] Task item 2\n\n```rust\nfn main() { println!(\"Fast!\"); }\n```\n\n| H1 | H2 |\n|---|---|\n| C1 | C2 |\n\n> Blockquote text here.\n";
    let medium_doc = sample_md.repeat(500); // ~100 KB document
    let mut parser = MarkdownParser::new();

    let t0 = Instant::now();
    parser.parse_full(black_box(&medium_doc));
    let full_parse_us = t0.elapsed().as_micros();
    println!("  • Full Parse of ~100 KB Markdown:     {:>6} µs ({:.2} MB/s)", full_parse_us, (medium_doc.len() as f64 / 1_000_000.0) / (full_parse_us as f64 / 1_000_000.0));

    // Incremental reparsing with state convergence
    let t0 = Instant::now();
    let num_inc = 5_000;
    for _ in 0..num_inc {
        parser.reparse_incremental(black_box(&medium_doc), black_box(10));
    }
    let inc_us = t0.elapsed().as_micros();
    println!("  • 5,000 Incremental Re-parses:        {:>6} µs ({:.2} µs/op, early convergence)\n", inc_us, (inc_us as f64) / num_inc as f64);

    // -------------------------------------------------------------------------
    // 4. Vector Typography & Font Engine (lekhni-raster)
    // -------------------------------------------------------------------------
    println!("--- [4] VECTOR TYPOGRAPHY & FONT CACHE (lekhni-raster) ---");
    let t0 = Instant::now();
    let fonts = FontCollection::load_default().expect("fonts load");
    let font_init_us = t0.elapsed().as_micros();
    println!("  • Font Engine Boot & Glyph Atlas:     {:>6} µs", font_init_us);

    let t0 = Instant::now();
    let num_meas = 50_000;
    for _ in 0..num_meas {
        let (w, h) = fonts.editor.measure_text(black_box("fn calculate_budget(ops: usize) -> u64;"));
        black_box((w, h));
    }
    let meas_us = t0.elapsed().as_micros();
    println!("  • 50,000 Text Measurements:           {:>6} µs ({:.2} ns/op, Zero-alloc)", meas_us, (meas_us as f64 * 1000.0) / num_meas as f64);

    let mut fb = vec![0xFF_28_2C_34u32; 1280 * 800];
    let mut canvas = Canvas::new(&mut fb, 1280, 800);
    let t0 = Instant::now();
    let num_draws = 10_000;
    for _ in 0..num_draws {
        let adv = fonts.editor.draw_text(&mut canvas, black_box("const BUFFER_CAPACITY: usize = 1024 * 1024;"), black_box(40), black_box(40), black_box(0xFF_AB_B2_BF));
        black_box(adv);
    }
    let draw_us = t0.elapsed().as_micros();
    println!("  • 10,000 Subpixel Text Blits:         {:>6} µs ({:.2} ns/string, {:.2} Mchars/sec)\n",
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
        canvas.fill_rect(black_box(Rect::new(x as i32, y as i32, 120, 24)), black_box(0xFF_61_AF_EF));
    }
    let fill_us = t0.elapsed().as_micros();
    println!("  • 20,000 Clipped fill_rect Calls:     {:>6} µs ({:.2} ns/rect, {:.2} Mpixels/sec)",
        fill_us,
        (fill_us as f64 * 1000.0) / num_rects as f64,
        (num_rects as f64 * 120.0 * 24.0) / (fill_us as f64)
    );

    // 4K (3840x2160) Full Frame Redraw
    let mut fb_4k = vec![0xFF_1E_1E_1Eu32; 3840 * 2160];
    let mut canvas_4k = Canvas::new(&mut fb_4k, 3840, 2160);
    let t0 = Instant::now();
    let num_4k_frames = 100;
    for _ in 0..num_4k_frames {
        canvas_4k.clear(black_box(0xFF_1E_1E_1E));
        canvas_4k.fill_rect(black_box(Rect::new(0, 0, 3840, 48)), black_box(0xFF_28_2C_34));
        canvas_4k.fill_rect(black_box(Rect::new(0, 48, 280, 2112)), black_box(0xFF_21_25_2B));
    }
    let redraw_4k_us = t0.elapsed().as_micros();
    println!("  • 100 Full 4K (3840x2160) Redraws:    {:>6} µs ({:.2} ms/frame)\n",
        redraw_4k_us,
        (redraw_4k_us as f64 / num_4k_frames as f64) / 1000.0
    );

    // -------------------------------------------------------------------------
    // 6. Trigram Search Engine Subcomponent (lekhni-index)
    // -------------------------------------------------------------------------
    println!("--- [6] TRIGRAM FULL-TEXT SEARCH (lekhni-index) ---");
    let mut trigram_idx = TrigramIndex::new();
    let t0 = Instant::now();
    for i in 0..10_000 {
        let content = format!("Notebook entry {} with Rust systems code, memory safety, and SIMD layout optimizations.", i);
        trigram_idx.index_doc(i, black_box(content.as_bytes()));
    }
    let index_docs_us = t0.elapsed().as_micros();
    println!("  • Index 10,000 Documents:             {:>6} µs ({:.2} µs/doc)", index_docs_us, index_docs_us as f64 / 10_000.0);

    let t0 = Instant::now();
    let num_queries = 2_000;
    for _ in 0..num_queries {
        let hits = trigram_idx.query_candidates(black_box(b"optimizations"));
        black_box(hits);
    }
    let query_us = t0.elapsed().as_micros();
    println!("  • 2,000 Full Trigram Queries:         {:>6} µs ({:.2} µs/query, {:.2} queries/sec)\n",
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
        let layouts = PreviewRenderer::compute_layouts(black_box(&parser.blocks), black_box(sample_md), black_box(800));
        black_box(layouts);
    }
    let layout_us = t0.elapsed().as_micros();
    println!("  • 5,000 Block Layout Passes:          {:>6} µs ({:.2} µs/pass)", layout_us, layout_us as f64 / num_layouts as f64);

    let t0 = Instant::now();
    let num_prev_draws = 1_000;
    for _ in 0..num_prev_draws {
        let targets = PreviewRenderer::render_preview_with_fonts(
            &mut canvas,
            black_box(&parser.blocks),
            black_box(sample_md),
            black_box(0),
            black_box(800),
            black_box(600),
            Some(&fonts),
        );
        black_box(targets);
    }
    let prev_render_us = t0.elapsed().as_micros();
    println!("  • 1,000 Preview Render Passes:        {:>6} µs ({:.2} µs/frame)\n",
        prev_render_us,
        prev_render_us as f64 / num_prev_draws as f64
    );

    // -------------------------------------------------------------------------
    // 8. Crash Recovery Journaling Subcomponent (lekhni-store)
    // -------------------------------------------------------------------------
    println!("--- [8] CRASH RECOVERY JOURNALING & DURABILITY (lekhni-store) ---");
    let tmp_dir = std::env::temp_dir().display().to_string();
    let test_bytes = b"# Unsaved draft state snapshot for crash resilience";
    let t0 = Instant::now();
    let num_journals = 100;
    for _ in 0..num_journals {
        let res = RecoveryJournal::write_snapshot(&StdFs, &tmp_dir, "bench_test", black_box(test_bytes));
        black_box(res).expect("snapshot write");
    }
    let journal_us = t0.elapsed().as_micros();
    println!("  • 100 Atomic Snapshots with fsync:    {:>6} µs ({:.2} ms/flush, hardware durable)", journal_us, (journal_us as f64 / num_journals as f64) / 1000.0);
    let _ = RecoveryJournal::discard_snapshot(&StdFs, &tmp_dir, "bench_test");

    // -------------------------------------------------------------------------
    // 9. Core Keystroke Pipeline Latency (lekhni-shell-desktop)
    // -------------------------------------------------------------------------
    println!("\n--- [9] CORE KEYSTROKE PIPELINE (lekhni-shell-desktop) ---");
    let mut desktop = DesktopEditor::new(b"# Live Document\nTesting end-to-end typing pipeline.\n", 1280, 800);
    let mut latencies = Vec::with_capacity(1_000);
    for _ in 0..1_000 {
        let lat = desktop.handle_keystroke(black_box(b"a"));
        latencies.push(lat);
    }
    let min_lat = latencies.iter().copied().min().unwrap_or(0) as u64;
    let max_lat = latencies.iter().copied().max().unwrap_or(0) as u64;
    let avg_lat = (latencies.iter().copied().sum::<u128>() / latencies.len() as u128) as u64;

    println!("  • 1,000 Keystrokes (Buffer + Damage + Layout + Blit):");
    println!("    - Average Pipeline Latency:  {:>5} µs ({:.3} ms)", avg_lat, avg_lat as f64 / 1000.0);
    println!("    - Minimum Pipeline Latency:  {:>5} µs ({:.3} ms)", min_lat, min_lat as f64 / 1000.0);
    println!("    - Maximum Pipeline Latency:  {:>5} µs ({:.3} ms)", max_lat, max_lat as f64 / 1000.0);
    println!("    - Typing Latency Budget:     < 8,000 µs (8.0 ms)");
    println!("    - Budget Margin:             {:.1}x FASTER than budget", 8000.0 / avg_lat.max(1) as f64);

    println!("\n========================================================================");
    println!("          ALL SUBCOMPONENT SPEED CHECKS COMPLETED                        ");
    println!("========================================================================");
}
