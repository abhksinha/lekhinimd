# Lekhni Development Milestones & Roadmap

- [x] ~~**M0: Foundation & Text Engine**~~
  - [x] ~~Workspace configuration and release profile optimization (`opt-level=3`, `lto="fat"`, `codegen-units=1`, `panic="abort"`).~~
  - [x] ~~`lekhni-base` (`no_std`): `FixedVec`, word-aligned `BitSet`, `Fixed26_6` fixed-point arithmetic, 64-bit content hash, branchless UTF-8 lookup tables.~~
  - [x] ~~`lekhni-text` (`no_std`): 16-byte cache-aligned `Piece` descriptor, append-only `PieceTable` with in-place typing coalescing.~~
  - [x] ~~`lekhni-text`: `Cursor` and `Selection` primitives.~~
  - [x] ~~`lekhni-text`: Line index resolution (`offset_to_line`, `line_to_offset`).~~
  - [x] ~~`lekhni-text`: `UndoManager` with piece descriptor snapshots.~~
  - [x] ~~Integration and unit test suite passing for both `alloc` and pure `no_std` builds.~~

- [x] ~~**M1: Incremental Markdown Parser (`lekhni-md`)**~~
  - [x] ~~Structure of Arrays (SoA) block table (`kind: u8[]`, `start: u32[]`, `end: u32[]`, `depth: u8[]`, `flags: u8[]`).~~
  - [x] ~~Line checkpointing system (packed parser state every 64 lines in one `u64`).~~
  - [x] ~~Incremental re-parse algorithm (restart from checkpoint `<= dirty_line`, block rollback and splice).~~
  - [x] ~~Lazy inline span parser for visible blocks (packed 32-bit descriptors).~~
  - [x] ~~CommonMark block classification and verification tests passing in both `alloc` and pure `no_std`.~~

- [x] ~~**M2: Software Rasterizer, Glyph Atlas & Desktop Shell**~~
  - [x] ~~`lekhni-raster`: CPU software blitter with integer premultiplied RGBA8 blending and damage rect clipping.~~
  - [x] ~~Glyph atlas: 8-bit coverage, shelf bin-packer.~~
  - [x] ~~`lekhni-shell`: Trait definitions (`Fs`, `Clipboard`, `Watcher`, `Clock`, `Presenter`, `Dialogs`, `Fonts`).~~
  - [x] ~~`lekhni-shell-desktop`: Desktop shell and latency proof harness proving < 8 ms keystroke latency.~~

- [x] ~~**M3: IME Verification & Complex Script Shaping**~~
  - [x] ~~Preedit string inline rendering and cursor management (`ImeManager`, `Preedit`, `ImeEvent`).~~
  - [x] ~~IME event translation and buffer commitment integration.~~
  - [x] ~~Complex script shaper classifier (Devanagari/Hindi, Arabic, CJK) and `Shaper` trait.~~
  - [x] ~~Pass IME verification matrix gate (Devanagari phonetic/InScript, CJK commits).~~

- [x] ~~**M4: Preview Pane & Editing Interactions**~~
  - [x] ~~Live Markdown preview renderer (viewport-only rendering in `PreviewRenderer`).~~
  - [x] ~~Smooth integer scrolling and damage-rect blit shifts (`ScrollState`).~~
  - [x] ~~Mouse and keyboard text selection, clipboard integration (`SelectionHandler`).~~
  - [x] ~~Undo/redo user interface integration.~~

- [x] ~~**M5: Notebook Storage & Sync Management**~~
  - [x] ~~`lekhni-store`: Directory-based notebook model (`NotebookModel`, `NotebookPage`).~~
  - [x] ~~Atomic file save protocol (`save_atomic` over shell `Fs` trait).~~
  - [x] ~~Crash recovery journal in `.lekhni/recover/` (`RecoveryJournal`).~~
  - [x] ~~External file modification tracking and conflict detection (`FileGeneration`, `check_external_edit`).~~

- [ ] **M6: Full Desktop UI & Navigation**
  - [ ] Virtualized notebook file tree and page list.
  - [ ] 3-pane fixed layout with draggable splitters.
  - [ ] Focus manager, tab navigation, and static shortcut table.
  - [ ] Color theme configuration.

- [ ] **M7: Search & Indexing Engine**
  - [ ] `lekhni-index`: Trigram search postings (SoA, compressed).
  - [ ] Document metadata table (AoS) and link graph.
  - [ ] Search query UI with real-time candidate filtering.

- [ ] **M8: Accessibility Bridge**
  - [ ] Accessibility tree emission in `lekhni-ui`.
  - [ ] AccessKit integration in `lekhni-shell-desktop`.

- [ ] **M9: GPU Acceleration Backend**
  - [ ] `wgpu` presenter backend for display list instanced quads and atlas textures.
  - [ ] Comparative benchmark: software rasterizer vs `wgpu` at high-DPI/high-refresh.

- [ ] **M10: Cross-Platform Ports**
  - [ ] Android shell (`lekhni-shell-android`).
  - [ ] iOS shell (`lekhni-shell-ios`).
  - [ ] WebAssembly shell (`lekhni-shell-web`) using OPFS.
