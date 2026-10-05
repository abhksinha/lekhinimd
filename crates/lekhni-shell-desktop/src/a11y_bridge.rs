//! Desktop accessibility bridge to assistive technologies (AccessKit).
//!
//! Lazily translates Lekhni UI accessibility tree snapshots into platform
//! accessibility representations.

use lekhni_ui::a11y::{A11yNode, A11yRole, A11yTree};

/// Desktop accessibility provider bridging to assistive technologies.
pub struct DesktopA11yBridge {
    pub is_assistive_tech_active: bool,
    pub last_tree: Option<A11yTree>,
}

impl Default for DesktopA11yBridge {
    fn default() -> Self {
        Self::new()
    }
}

impl DesktopA11yBridge {
    pub const fn new() -> Self {
        Self {
            is_assistive_tech_active: false,
            last_tree: None,
        }
    }

    /// Updates the accessibility tree only when assistive technology is actively listening.
    pub fn update_tree(&mut self, tree: A11yTree) {
        if self.is_assistive_tech_active {
            self.last_tree = Some(tree);
        }
    }

    /// Builds a sample desktop accessibility tree for the editor.
    pub fn build_editor_a11y_tree(
        window_width: i32,
        window_height: i32,
        editor_text: &str,
    ) -> A11yTree {
        let mut tree = A11yTree::new(1);

        // Root window node
        tree.add_node(A11yNode {
            id: 1,
            role: A11yRole::Window,
            bounds: lekhni_raster::damage::Rect::new(0, 0, window_width, window_height),
            name: Some("Lekhni Markdown Editor".into()),
            value: None,
            children: vec![2],
        });

        // Editor textarea node
        tree.add_node(A11yNode {
            id: 2,
            role: A11yRole::MultilineTextInput,
            bounds: lekhni_raster::damage::Rect::new(0, 0, window_width, window_height),
            name: Some("Document Text".into()),
            value: Some(editor_text.into()),
            children: vec![],
        });

        tree
    }
}
