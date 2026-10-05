//! Cursor and selection management with byte offsets and visual affinity.

/// Cursor position within a text buffer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Cursor {
    /// Byte offset in the buffer (guaranteed to be on a valid UTF-8 character boundary).
    pub byte_offset: usize,
    /// 0-indexed line number.
    pub line: usize,
    /// 0-indexed UTF-8 or visual column.
    pub column: usize,
}

impl Cursor {
    #[inline(always)]
    pub const fn new(byte_offset: usize, line: usize, column: usize) -> Self {
        Self {
            byte_offset,
            line,
            column,
        }
    }
}

/// Selection range defined by an anchor (start of selection interaction) and active cursor.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Selection {
    /// Stationary point of the selection.
    pub anchor: Cursor,
    /// Moving point / head of the selection.
    pub head: Cursor,
}

impl Selection {
    #[inline(always)]
    pub const fn collapsed(cursor: Cursor) -> Self {
        Self {
            anchor: cursor,
            head: cursor,
        }
    }

    #[inline(always)]
    pub fn is_collapsed(&self) -> bool {
        self.anchor.byte_offset == self.head.byte_offset
    }

    #[inline(always)]
    pub fn start(&self) -> Cursor {
        if self.anchor.byte_offset <= self.head.byte_offset {
            self.anchor
        } else {
            self.head
        }
    }

    #[inline(always)]
    pub fn end(&self) -> Cursor {
        if self.anchor.byte_offset <= self.head.byte_offset {
            self.head
        } else {
            self.anchor
        }
    }

    #[inline(always)]
    pub fn byte_range(&self) -> (usize, usize) {
        if self.anchor.byte_offset <= self.head.byte_offset {
            (self.anchor.byte_offset, self.head.byte_offset)
        } else {
            (self.head.byte_offset, self.anchor.byte_offset)
        }
    }
}
