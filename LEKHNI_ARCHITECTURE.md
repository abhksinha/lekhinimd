# Lekhni Architecture v2

Pure Rust Markdown notebook editor and viewer. Native. No webview. No Tauri. 1980s discipline: small, fixed, measured.

## 0. Honest scope of "pure Rust, no_std"

- Everything that can be `no_std` IS `no_std`: core, parser, index, layout, widgets, display list, software rasterizer.
- OS contact (window, input, files, clipboard, GPU) needs `std` or raw syscalls. Confined to thin `shell-*` crates.
- Rule: `std` allowed only in shell crates. CI fails if a core crate pulls `std`.
- Third-party deps: each must be `no_std` (+ optional `alloc`) in core crates. Audit before adding. Vendor and pin.
- Shaping for Indic/Arabic needs a real shaper. Verify current `no_std` status of rustybuzz / ttf-parser. If not clean, shaper lives in shell behind a trait.

## 1. Goals and budgets

| Target | Budget (measure, then tighten) |
|---|---|
| Keystroke to pixels | <= 1 frame, ideally < 8 ms |
| Cold start to usable | < 100 ms |
| Idle CPU | 0% (no timers, no polling) |
| Idle RSS, 1 notebook open | < 30 MB |
| Open 10 MB .md | < 50 ms to first screen |
| Binary size, desktop | < 5 MB stripped |
| Heap allocs per keystroke | 0 in steady state |

Every budget has a benchmark in `bench/`. No benchmark, no claim.

## 2. Principles

1. Zero-work: compute nothing without a visible or requested outcome.
2. Files are truth: `.md` on disk. Everything else disposable.
3. No heap in hot path: arenas, fixed-capacity vectors, reuse buffers.
4. Integer and fixed-point math in layout and raster. Floats only in shaping/metrics input.
5. Tables over branches: lookup tables for char class, UTF-8 decode, blend.
6. Dirty ranges plus generation counters everywhere. Stale = recompute lazily.
7. One pipeline: editor and UI widgets share scene, atlas, damage tracker.
8. Abstraction only at platform boundary (traits). Elsewhere concrete types.
9. Measure first. Pack bits only where profile says so.

## 3. Crate layout

```
lekhni/
  crates/
    lekhni-base        no_std  arena, fixed vec, bitset, fixed-point, hash, utf8 tables
    lekhni-text        no_std  piece table, line index, undo, selection, cursor, grapheme/word
    lekhni-md          no_std  incremental block parser, inline spans, style runs
    lekhni-index       no_std  trigram search, title/tag/link index, SoA, mmap-friendly format
    lekhni-store       no_std  notebook model, paths, atomic-save plan, external-edit logic (over Shell trait)
    lekhni-layout      no_std  line wrap, shaping cache, metrics, 3-pane fixed layout
    lekhni-ui          no_std  widgets, focus, hit-test, scene, display list, damage, a11y tree
    lekhni-raster      no_std  software rasterizer, glyph atlas, blend, clip
    lekhni-shell       no_std  traits only: Fs, Clipboard, Watcher, Clock, Presenter, Ime, Dialogs
    lekhni-shell-desktop  std  winit + (softbuffer | wgpu) + fs + watcher + clipboard + AccessKit bridge
    lekhni-shell-android  std  later
    lekhni-shell-ios      std  later
    lekhni-shell-web      std  later (wasm, OPFS / File System Access)
  bench/
  fuzz/
  tests/
```

Dependency direction: base <- text <- md <- layout <- ui. store, index depend on base/text/md. Shell crates depend on everything. Core never depends on shell crates, only on `lekhni-shell` traits.

## 4. Data structures

### 4.1 Buffer (lekhni-text)
- Piece table: immutable original (mmap'd file bytes) + append-only add buffer.
- Pieces in a flat array with a sum tree (B-tree or Fenwick) over byte length and newline count.
- Line index: sum tree gives line to byte offset in O(log n). No separate full-line array.
- Edit = split/insert piece. Typing at cursor extends last piece in place (coalesce).
- Undo/redo: piece-list snapshots are cheap (only descriptors). Group by time/word boundary. Memory only by default; optional `.lekhni/undo` persistence later.
- Rejected: gap buffer (poor for multi-cursor, big undo), full rope (heavier, more allocs).
- UTF-8 internally. Cursor stored as byte offset plus cached column and grapheme boundary.

### 4.2 Parse output (lekhni-md)
- Block table (SoA): `kind: u8[]`, `start: u32[]`, `end: u32[]`, `depth: u8[]`, `flags: u8[]`.
- Inline spans per visible block only (lazy): packed `u32` = offset delta + 4-bit style + 4-bit class.
- Style bits: bold, italic, code, link, strike, heading level (3 bits), quote, list marker.
- Checkpoints every N lines (N ~ 64): packed parser state (fence open + fence char/len, list stack depth, quote depth, html-block flag) in one `u64`.

### 4.3 Index (lekhni-index)
- Trigram postings: SoA, sorted `u32` doc ids, delta + varint compressed, mmap'd from `.lekhni/`.
- Doc table (AoS, complete record): id, path hash, mtime, size, title offset, tag range, link range.
- Link graph: edge list `(src u32, dst u32)` sorted both ways for backlinks.
- Index file: magic + format version + notebook generation. Mismatch = discard and rebuild lazily.

### 4.4 Scene (lekhni-ui)
- Display list: flat `Vec<Cmd>` reused each frame. Cmd = FillRect, GlyphRun, Line, ClipPush/Pop. 16-byte packed commands.
- Retained per-pane lists with generation. Unchanged pane = reuse last list, zero rebuild.

## 5. Pipeline per keystroke

```
OS key/IME event
  -> shell translates to Event (no alloc)
  -> ui routes to focused widget (editor)
  -> text: apply edit (piece table), bump generation, mark dirty byte range
  -> md: reparse from nearest checkpoint <= dirty line, stop when state converges with old checkpoint
  -> layout: re-wrap only affected lines (shaping cache keyed by line content hash + width + font)
  -> ui: rebuild display list for dirty lines only, compute damage rects
  -> raster: draw damage rects only
  -> shell Presenter: present only damaged region
  -> store: mark page dirty, schedule save (debounced, via deadline not timer thread)
```

Rules:
- No event coalescing for key input. Pointer move may coalesce.
- Redraw on demand. Event loop blocks when idle. Animations (caret blink) use a single deadline wake, off when unfocused.
- Preview pane updates for visible region only, one frame behind max, never blocks typing.

## 6. Incremental Markdown parsing

- Block-level reparse, not token-level.
- On edit at line L: restart from checkpoint <= L, parse forward until produced state at a checkpoint equals old state, then splice.
- Fence open/close and list continuation can cascade: converge check handles it.
- Inline parse only for lines in or near viewport. Cache by `(block id, generation)`.
- CommonMark + GFM tables, task lists, strikethrough, footnotes later. Spec test suite in `tests/`. Fuzz parser with cargo-fuzz: no panic, no unbounded time.
- Parser must be linear. Pathological input guards: nesting depth cap, link-ref cap, line length soft cap.

## 7. Text, shaping, fonts

- Two paths:
  - Fast path: simple scripts (Latin etc.), cmap lookup + advance table, kerning optional. No shaper call.
  - Complex path: Devanagari, Arabic, other Indic via shaper (rustybuzz or equivalent, behind `Shaper` trait). Required for Hindi.
- Shaping cache per line: key `(hash, font id, size, width)`. LRU, fixed capacity, arena-backed.
- Font parsing: ttf-parser class crate (no_std). Fonts mmap'd, parsed lazily, table offsets cached.
- Fallback chain defined in a small static table. Missing glyph = next font. Cached per codepoint block.
- Glyph atlas: grayscale 8-bit coverage, subpixel x quantized to 1/4 px, shelf packer, fixed size (e.g. 2048x2048), evict by LRU row.
- Bundled font: one compact UI font + one mono, subset. Rest system fonts via shell.
- Bidi: UAX#9 implementation (no_std crate or own). Only run if line contains RTL class.

## 8. Rendering backends

Single `Presenter` trait. Two implementations, benchmark decides default per platform.

1. Software (default candidate): `lekhni-raster` draws damage rects into a CPU framebuffer, shell blits with softbuffer-class crate. Lowest latency for small damage, no GPU init cost, no driver variance, tiny binary.
2. wgpu: display list to instanced quads + atlas texture. Better for high-DPI full-window redraws, scrolling at 4K/120Hz+, web (WebGPU). Present mode Mailbox/Immediate where available.

Raster details:
- Premultiplied RGBA8, integer blend with `(a*b + 127) / 255` approximation via table or shift trick.
- Scanline-free: axis-aligned rects and glyph blits only. No path rasterizer in v1 (icons = pre-baked bitmaps or simple line primitives).
- SIMD: use `core::arch` behind feature flags (SSE2/AVX2/NEON), scalar fallback. `portable_simd` only if stable at that time.
- Scroll: blit-shift framebuffer region, redraw only newly exposed lines.

## 9. UI (own widgets, no toolkit)

Layout: fixed 3 panes. Notebook sidebar | page list | editor/preview. Splitters adjustable. Integer pixel layout, computed on resize only.

Widgets (complete list for v1):
- VirtualList (page list, search results): only visible rows instantiated.
- Tree (notebook sidebar): virtualized, keyboard nav, inline rename.
- ScrollArea: shared scroll physics, integer positions.
- TextLine (single-line input): search box, rename. Reuses editor text engine.
- Button / Menu / Popup: minimal.
- Splitter.
- EditorView and PreviewView: the two custom heavy widgets.

Infrastructure:
- Focus manager, tab order, shortcut table (static, remappable via config file).
- Hit testing against flat rect table (SoA), no tree walk.
- Theme: static struct of colors and metrics, swap = one pointer, mark all dirty.
- Drag and drop: shell provides events, ui handles reorder/move notebook items.

## 10. Input, IME, accessibility

IME (priority one, test before other widgets):
- Core holds preedit string, cursor within preedit, committed text. Rendered inline with underline style.
- Shell maps winit IME events (preedit, commit, enable/disable, cursor area) to core events. Report caret rect back to OS for candidate window placement.
- Test matrix: Hindi (Devanagari phonetic and InScript), Chinese pinyin, Japanese, Korean, dead keys, emoji picker, voice input.

Accessibility:
- `lekhni-ui` can emit an accessibility tree (nodes, roles, bounds, text ranges).
- Desktop shell bridges to AccessKit. Tree built lazily only when assistive tech is attached. Zero cost otherwise.
- Editor exposes text with selection and line/word granularity for screen readers.

Keyboard: full operation without mouse. Vim/Emacs-style keymaps = optional table packs, not core.

## 11. Storage and files

```
Notebook/
  .lekhni/            disposable (gitignore it)
    index.bin         search + metadata, versioned, rebuild on mismatch
    state.bin         UI state, last open page, scroll, layout
    cache/            shaping/render caches (optional)
    undo/             optional persisted undo
  Page One.md
  sub-notebook/
    Page Two.md
  assets/             or per-page folders, relative refs only
    diagram.png
```

- Source of truth: plain `.md`. Delete `.lekhni/` and nothing is lost.
- Atomic save: write temp file in same dir, fsync file, rename over target, fsync dir (where supported).
- Crash recovery: periodic journal of dirty buffers in `.lekhni/recover/`. Offered on next open.
- External edits: track `(mtime, size, content hash of last-saved)` as generation. Watcher event or focus-regain check. If buffer clean: reload silently. If dirty: conflict prompt (keep mine / take theirs / diff).
- Rename/move: update relative links in affected pages (opt-in, preview diff first).
- Attachments: normal files, relative links. Optional content-addressed dedup later.
- Front matter (YAML) preserved byte-for-byte unless edited.
- Large files: mmap original, piece table never copies it.
- Sync: user's choice (git, Syncthing, cloud folder). `.lekhni/` safe to exclude. No lock files in notebook root.

## 12. Search and indexing

- Lazy and opportunistic: index a file when opened, when changed, or during idle after input quiet for N ms. Never background-spin.
- Queries: trigram prefilter, then verify with substring/regex on candidates. Title/tag/link queries hit doc table directly.
- Incremental: per-file postings segment, merged lazily. Deleted/changed files tombstoned by generation.
- Budget: first-run index of 10k pages must be interruptible and resumable.

## 13. Shell trait boundary

```rust
// lekhni-shell (no_std): traits only
trait Fs        { read_map, write_atomic, list_dir, stat, remove, rename }
trait Watcher   { subscribe(path), poll() }
trait Clipboard { get_text, set_text }
trait Clock     { now_ms, set_deadline }
trait Presenter { size, scale, present(damage, pixels|display_list) }
trait Dialogs   { open_dir, confirm }
trait Fonts     { enumerate, load_bytes, fallback_for(codepoint) }
```

- Core never blocks on shell. IO requests return handles or complete synchronously only for tiny ops. Large reads via mmap.
- Web shell: Fs backed by OPFS / File System Access API. Same core.

## 14. Platform matrix

| Platform | Window/input | Present | Notes |
|---|---|---|---|
| Windows | winit | softbuffer / wgpu (DX12) | IME via TSF through winit |
| macOS | winit | softbuffer / wgpu (Metal) | NSTextInputClient through winit |
| Linux | winit (X11, Wayland) | softbuffer / wgpu (Vulkan) | IBus/fcitx via winit, Wayland text-input |
| Android | winit or native-activity | wgpu or software | soft keyboard, selection handles manual |
| iOS | winit | wgpu (Metal) | same as Android class of work |
| Web | wasm + canvas | wgpu (WebGPU) / 2D canvas blit | no real fs, OPFS |

winit and OS bindings are `std`. They stay in shell crates only.

## 15. Build and performance engineering

- Profile: `opt-level=3`, `lto=fat`, `codegen-units=1`, `panic=abort`, `strip`.
- Core crates: `#![no_std]`, `#![forbid(unsafe_code)]` except audited modules (mmap view, SIMD blit). Each `unsafe` has a safety comment and a test.
- Allocator: bump arena per frame, pool for long-lived. Desktop shell may use a small global allocator (mimalloc-class) for the `alloc` parts.
- Bench suite (criterion-like, `no_std` friendly harness in shell): keystroke path, parse reflow, scroll, search, open-large-file, atlas churn.
- Latency harness: timestamp event receipt and present call. Optional external photodiode rig for true end-to-end.
- Fuzz: parser, piece table (edit sequences vs reference Vec<u8>), index format loader, UTF-8 boundary logic.
- Property tests: undo/redo round-trip, incremental parse == full parse for random edits.
- CI gates: no `std` in core crates, binary size budget, bench regression threshold.

## 16. Milestones

1. M0: `lekhni-base`, `lekhni-text` + tests/fuzz. Piece table, undo, line index.
2. M1: `lekhni-md` incremental parser, equals full parse under property test, CommonMark suite passing.
3. M2: software raster + glyph atlas + desktop shell. Editor widget only. Prove typing latency.
4. M3: IME proof (Hindi + CJK) and complex-script shaping. Gate: do not continue until passing.
5. M4: Preview widget, scroll, selection, clipboard, undo UI.
6. M5: Store: notebooks, atomic save, watcher, crash recovery.
7. M6: Sidebar, page list, splitters, shortcuts (virtualized).
8. M7: Search index + search UI.
9. M8: AccessKit bridge.
10. M9: wgpu presenter, benchmark vs software, pick defaults.
11. M10: Android, iOS, web shells.

## 17. Risks (ranked)

1. IME and complex-script correctness. Mitigation: M3 gate, native test devices.
2. Accessibility effort. Mitigation: tree designed from day one in `lekhni-ui`, AccessKit bridge at M8.
3. `no_std` dependency gaps (shaper, bidi, font parsing). Mitigation: trait seams, vendor or write small subsets.
4. Software raster at 4K/high refresh scrolling. Mitigation: blit-shift, damage rects, wgpu backend as alternative.
5. Mobile input and selection UX. Mitigation: defer, keep core event model platform-neutral.
6. Scope creep: plugins, themes engine, rich embeds. Mitigation: non-goals below.

## 18. Non-goals (v1)

- Plugin system, scripting.
- WYSIWYG rich editing (source editing + live preview only).
- Built-in sync service.
- Collaborative editing.
- Proprietary database.
- Generic widget toolkit for others.

## 19. Open decisions

- Default presenter per platform: software vs wgpu (benchmark at M9).
- Shaper choice and `no_std` feasibility (verify at M3).
- Persist undo history or not.
- Notebook model: pure directories vs optional `notebook.toml` per folder for ordering/icons.
- Link syntax: standard Markdown links only, or add `[[wikilinks]]` resolution.
- Config format: single TOML file in `.lekhni/` or user config dir.
