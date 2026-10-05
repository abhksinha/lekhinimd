//! Fixed 3-pane layout engine.
//!
//! Follows Section 9 of LEKHNI_ARCHITECTURE:
//! Integer pixel layout, computed on resize only. Splitters adjustable.

#![no_std]

use lekhni_raster::damage::Rect;

/// Represents the geometric bounds of the 3 workspace panes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct WorkspaceLayout {
    pub menu_bar_rect: Rect,
    pub sidebar_rect: Rect,
    pub splitter_1_rect: Rect,
    pub page_list_rect: Rect,
    pub splitter_2_rect: Rect,
    pub editor_rect: Rect,
    pub preview_rect: Rect,
    pub status_bar_rect: Rect,
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
    compute_workspace_layout_with_chrome(
        window_width,
        window_height,
        sidebar_width,
        page_list_width,
        splitter_width,
        show_preview,
        0,
        0,
    )
}

/// Computes workspace panes including top menu bar and bottom status bar.
#[allow(clippy::too_many_arguments)]
pub fn compute_workspace_layout_with_chrome(
    window_width: i32,
    window_height: i32,
    sidebar_width: i32,
    page_list_width: i32,
    splitter_width: i32,
    show_preview: bool,
    menu_bar_height: i32,
    status_bar_height: i32,
) -> WorkspaceLayout {
    let menu_bar_rect = Rect::new(0, 0, window_width, menu_bar_height);
    let status_bar_y = (window_height - status_bar_height).max(0);
    let status_bar_rect = Rect::new(0, status_bar_y, window_width, status_bar_height);

    let content_y = menu_bar_height;
    let content_h = (window_height - menu_bar_height - status_bar_height).max(0);

    let sb_w = sidebar_width.max(120).min(window_width / 2);
    let pl_w = page_list_width.max(150).min(window_width / 2);

    let sidebar_rect = Rect::new(0, content_y, sb_w, content_h);
    let splitter_1_rect = Rect::new(sb_w, content_y, splitter_width, content_h);

    let pl_x = sb_w + splitter_width;
    let page_list_rect = Rect::new(pl_x, content_y, pl_w, content_h);
    let splitter_2_rect = Rect::new(pl_x + pl_w, content_y, splitter_width, content_h);

    let editor_x = pl_x + pl_w + splitter_width;
    let remaining_w = (window_width - editor_x).max(100);

    let (editor_rect, preview_rect) = if show_preview {
        let half_w = remaining_w / 2;
        (
            Rect::new(editor_x, content_y, half_w, content_h),
            Rect::new(editor_x + half_w, content_y, remaining_w - half_w, content_h),
        )
    } else {
        (
            Rect::new(editor_x, content_y, remaining_w, content_h),
            Rect::default(),
        )
    };

    WorkspaceLayout {
        menu_bar_rect,
        sidebar_rect,
        splitter_1_rect,
        page_list_rect,
        splitter_2_rect,
        editor_rect,
        preview_rect,
        status_bar_rect,
    }
}
