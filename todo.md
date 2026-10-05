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

- [ ] **M2: Software Rasterizer, Glyph Atlas & Desktop Shell**
  - [ ] `lekhni-raster`: CPU software blitter with integer premultiplied RGBA8 blending and damage rect clipping.
  - [ ] Glyph atlas: 8-bit coverage, 1/4 pixel subpixel positioning, shelf packer.
  - [ ] `lekhni-shell`: Trait definitions (`Fs`, `Clipboard`, `Watcher`, `Clock`, `Presenter`, `Ime`, `Dialogs`).
  - [ ] `lekhni-shell-desktop`: `winit` + `softbuffer` integration.
  - [ ] Minimal editor view proving < 8 ms keystroke latency.

- [ ] **M3: IME Verification & Complex Script Shaping**
  - [ ] Preedit string inline rendering and cursor management.
  - [ ] IME event translation from `winit` (TSF/fcitx/IBus).
  - [ ] Complex script shaper integration (Devanagari/Hindi, CJK).
  - [ ] Pass IME verification matrix gate before downstream widget development.

- [ ] **M4: Preview Pane & Editing Interactions**
  - [ ] Live Markdown preview renderer (viewport-only rendering).
  - [ ] Smooth integer scrolling and damage-rect blit shifts.
  - [ ] Mouse and keyboard text selection, clipboard integration.
  - [ ] Undo/redo user interface integration.

- [ ] **M5: Notebook Storage & Sync Management**
  - [ ] `lekhni-store`: Directory-based notebook model.
  - [ ] Atomic file save (temp file -> fsync -> atomic rename).
  - [ ] Crash recovery journal in `.lekhni/recover/`.
  - [ ] External file modifications watcher and conflict detection.

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
