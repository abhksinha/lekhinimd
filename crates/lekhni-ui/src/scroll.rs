//! Integer scroll physics and blit-shift damage calculation.
//!
//! Follows Section 8 of LEKHNI_ARCHITECTURE:
//! Blit-shift framebuffer region on scroll, redraw only newly exposed lines.

use lekhni_raster::damage::Rect;

/// Integer scroll state for scrollable panes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct ScrollState {
    pub offset_y: i32,
    pub max_offset_y: i32,
    pub viewport_height: i32,
}

impl ScrollState {
    pub const fn new(viewport_height: i32) -> Self {
        Self {
            offset_y: 0,
            max_offset_y: 0,
            viewport_height,
        }
    }

    /// Sets content height and updates clamping bounds.
    pub fn update_content_height(&mut self, content_height: i32) {
        self.max_offset_y = (content_height - self.viewport_height).max(0);
        self.offset_y = self.offset_y.clamp(0, self.max_offset_y);
    }

    /// Applies a scroll delta, clamping to valid content bounds.
    /// Returns the actual scroll movement achieved.
    pub fn scroll_by(&mut self, delta_y: i32) -> i32 {
        let old = self.offset_y;
        self.offset_y = (self.offset_y + delta_y).clamp(0, self.max_offset_y);
        self.offset_y - old
    }

    /// Computes the newly exposed damage rectangle that requires redrawing after a scroll.
    pub fn compute_exposed_damage(&self, delta_y: i32, width: i32) -> Option<Rect> {
        if delta_y == 0 {
            return None;
        }

        if delta_y.abs() >= self.viewport_height {
            // Full redraw needed
            Some(Rect::new(0, 0, width, self.viewport_height))
        } else if delta_y > 0 {
            // Scrolled down: new content exposed at bottom
            let exposed_h = delta_y;
            let exposed_y = self.viewport_height - exposed_h;
            Some(Rect::new(0, exposed_y, width, exposed_h))
        } else {
            // Scrolled up: new content exposed at top
            let exposed_h = -delta_y;
            Some(Rect::new(0, 0, width, exposed_h))
        }
    }
}
