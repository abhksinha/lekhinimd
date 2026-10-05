use lekhni_shell_desktop::a11y_bridge::DesktopA11yBridge;
use lekhni_ui::a11y::A11yRole;

#[test]
fn test_a11y_tree_generation() {
    let tree = DesktopA11yBridge::build_editor_a11y_tree(800, 600, "# Hello A11y");
    assert_eq!(tree.nodes.len(), 2);
    assert_eq!(tree.nodes[0].role, A11yRole::Window);
    assert_eq!(tree.nodes[1].role, A11yRole::MultilineTextInput);
    assert_eq!(tree.nodes[1].value.as_deref(), Some("# Hello A11y"));

    let mut bridge = DesktopA11yBridge::new();
    assert!(!bridge.is_assistive_tech_active);
    bridge.update_tree(tree.clone());
    assert!(bridge.last_tree.is_none()); // Zero cost when inactive

    bridge.is_assistive_tech_active = true;
    bridge.update_tree(tree);
    assert!(bridge.last_tree.is_some());
}
