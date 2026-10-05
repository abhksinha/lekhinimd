//! Fixed 3-pane layout engine.
//!
//! Follows Section 9 of LEKHNI_ARCHITECTURE:
//! Integer pixel layout, computed on resize only. Splitters adjustable.

#![no_std]

use lekhni_raster::damage::Rect;

/// Represents the geometric bounds of the 3 workspace panes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct WorkspaceLayout {
    pub sidebar_rect: Rect,
    pub splitter_1_rect: Rect,
    pub page_list_rect: Rect,
    pub splitter_2_rect: Rect,
    pub editor_rect: Rect,
    pub preview_rect: Rect,
}

/// Computes workspace panes given window dimensions and splitter positions.
pub fn compute_workspace_layout(
    window_width: i32,
    window_height: i32,
    sidebar_width: i32,
    page_list_width: i32,
    splitter_width: i32,
    show_preview: bool,
) -> WorkspaceLayout {
    let sb_w = sidebar_width.max(120).min(window_width / 2);
    let pl_w = page_list_width.max(150).min(window_width / 2);

    let sidebar_rect = Rect::new(0, 0, sb_w, window_height);
    let splitter_1_rect = Rect::new(sb_w, 0, splitter_width, window_height);

    let pl_x = sb_w + splitter_width;
    let page_list_rect = Rect::new(pl_x, 0, pl_w, window_height);
    let splitter_2_rect = Rect::new(pl_x + pl_w, 0, splitter_width, window_height);

    let editor_x = pl_x + pl_w + splitter_width;
    let remaining_w = (window_width - editor_x).max(100);

    let (editor_rect, preview_rect) = if show_preview {
        let half_w = remaining_w / 2;
        (
            Rect::new(editor_x, 0, half_w, window_height),
            Rect::new(editor_x + half_w, 0, remaining_w - half_w, window_height),
        )
    } else {
        (
            Rect::new(editor_x, 0, remaining_w, window_height),
            Rect::default(),
        )
    };

    WorkspaceLayout {
        sidebar_rect,
        splitter_1_rect,
        page_list_rect,
        splitter_2_rect,
        editor_rect,
        preview_rect,
    }
}
