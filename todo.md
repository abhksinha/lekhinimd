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

- [x] ~~**M6: Full Desktop UI & Navigation**~~
  - [x] ~~`lekhni-layout`: 3-pane fixed layout with adjustable splitters (`compute_workspace_layout`).~~
  - [x] ~~Virtualized notebook file tree (`TreeView`, `TreeNode`).~~
  - [x] ~~Virtualized page list (`PageListView`, `PageListItem`).~~
  - [x] ~~Focus manager, tab navigation, and static shortcut handling (`FocusedPane`).~~
  - [x] ~~Color theme configuration with dark and light palettes (`Theme`).~~

- [x] ~~**M7: Search & Indexing Engine**~~
  - [x] ~~`lekhni-index`: Trigram search postings (SoA, fast candidate pre-filtering with `TrigramIndex`).~~
  - [x] ~~Document metadata table (`DocRecord`, `DocTable`) and bidirectional link graph (`LinkGraph`).~~
  - [x] ~~Search query matching with multi-trigram intersection.~~

- [x] ~~**M8: Accessibility Bridge**~~
  - [x] ~~Accessibility tree emission in `lekhni-ui` (`A11yNode`, `A11yRole`, `A11yTree`).~~
  - [x] ~~AccessKit integration in `lekhni-shell-desktop` (`DesktopA11yBridge`) with zero-overhead when assistive technology is inactive.~~

- [x] ~~**M9: GPU Acceleration Backend**~~
  - [x] ~~`DisplayList` and packed 16-byte `DisplayCmd` retaining generation counters.~~
  - [x] ~~`GpuPresenter` instanced quad vertex/index streamer.~~
  - [x] ~~Comparative benchmark validating software rasterizer damage blits vs GPU instanced pipeline at 1080p.~~

- [x] ~~**M10: Cross-Platform Ports**~~
  - [x] ~~Android shell (`lekhni-shell-android`): NDK presenter, native events, and soft-keyboard integration.~~
  - [x] ~~iOS shell (`lekhni-shell-ios`): UIKit presenter and touch handling.~~
  - [x] ~~WebAssembly shell (`lekhni-shell-web`): Canvas bridge and Origin Private File System (`OpfsFs`) storage.~~
