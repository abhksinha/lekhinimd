//! Focus management and keyboard navigation order.

/// Which pane or widget currently holds input focus.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum FocusedPane {
    Sidebar,
    PageList,
    #[default]
    Editor,
    Preview,
}

impl FocusedPane {
    /// Advances focus to the next pane in circular tab order.
    pub fn next(&self) -> Self {
        match self {
            FocusedPane::Sidebar => FocusedPane::PageList,
            FocusedPane::PageList => FocusedPane::Editor,
            FocusedPane::Editor => FocusedPane::Sidebar,
            FocusedPane::Preview => FocusedPane::Sidebar,
        }
    }

    /// Moves focus to the previous pane in circular shift-tab order.
    pub fn prev(&self) -> Self {
        match self {
            FocusedPane::Sidebar => FocusedPane::Editor,
            FocusedPane::PageList => FocusedPane::Sidebar,
            FocusedPane::Editor => FocusedPane::PageList,
            FocusedPane::Preview => FocusedPane::Editor,
        }
    }
}
