//! Damage rectangle tracking for partial window redrawing.
//!
//! Minimizes CPU-to-display presentation traffic by only redrawing
//! areas bounded by damage rectangles.

/// An integer rectangle for damage tracking and clipping.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl Rect {
    #[inline(always)]
    pub const fn new(x: i32, y: i32, width: i32, height: i32) -> Self {
        Self { x, y, width, height }
    }

    #[inline(always)]
    pub const fn right(&self) -> i32 {
        self.x + self.width
    }

    #[inline(always)]
    pub const fn bottom(&self) -> i32 {
        self.y + self.height
    }

    /// Computes intersection with another rectangle.
    pub fn intersect(&self, other: &Rect) -> Option<Rect> {
        let x1 = self.x.max(other.x);
        let y1 = self.y.max(other.y);
        let x2 = self.right().min(other.right());
        let y2 = self.bottom().min(other.bottom());

        if x2 > x1 && y2 > y1 {
            Some(Rect {
                x: x1,
                y: y1,
                width: x2 - x1,
                height: y2 - y1,
            })
        } else {
            None
        }
    }

    /// Computes bounding union of two rectangles.
    pub fn union(&self, other: &Rect) -> Rect {
        if self.width <= 0 || self.height <= 0 {
            return *other;
        }
        if other.width <= 0 || other.height <= 0 {
            return *self;
        }

        let x1 = self.x.min(other.x);
        let y1 = self.y.min(other.y);
        let x2 = self.right().max(other.right());
        let y2 = self.bottom().max(other.bottom());

        Rect {
            x: x1,
            y: y1,
            width: x2 - x1,
            height: y2 - y1,
        }
    }
}
