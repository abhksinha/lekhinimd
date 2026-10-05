use lekhni_layout::compute_workspace_layout;

#[test]
fn test_workspace_layout_computation() {
    let layout = compute_workspace_layout(1200, 800, 200, 250, 4, true);

    assert_eq!(layout.sidebar_rect.width, 200);
    assert_eq!(layout.splitter_1_rect.x, 200);
    assert_eq!(layout.page_list_rect.x, 204);
    assert_eq!(layout.page_list_rect.width, 250);
    assert_eq!(layout.splitter_2_rect.x, 454);
    assert!(layout.editor_rect.width > 0);
    assert!(layout.preview_rect.width > 0);
    assert_eq!(layout.preview_rect.x, layout.editor_rect.right());
}
