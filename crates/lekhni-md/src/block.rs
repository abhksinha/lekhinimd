//! Structure of Arrays (SoA) layout for Markdown blocks.
//!
//! Follows Section 4.2 of LEKHNI_ARCHITECTURE:
//! Block table (SoA): kind: u8[], start: u32[], end: u32[], depth: u8[], flags: u8[].

#[cfg(feature = "alloc")]
use alloc::vec::Vec;

/// Markdown block type tag.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum BlockKind {
    Paragraph = 0,
    Heading1 = 1,
    Heading2 = 2,
    Heading3 = 3,
    Heading4 = 4,
    Heading5 = 5,
    Heading6 = 6,
    FencedCode = 7,
    BlockQuote = 8,
    ListItem = 9,
    ThematicBreak = 10,
    HtmlBlock = 11,
    Table = 12,
}

impl BlockKind {
    #[inline(always)]
    pub fn from_u8(val: u8) -> Self {
        match val {
            1 => BlockKind::Heading1,
            2 => BlockKind::Heading2,
            3 => BlockKind::Heading3,
            4 => BlockKind::Heading4,
            5 => BlockKind::Heading5,
            6 => BlockKind::Heading6,
            7 => BlockKind::FencedCode,
            8 => BlockKind::BlockQuote,
            9 => BlockKind::ListItem,
            10 => BlockKind::ThematicBreak,
            11 => BlockKind::HtmlBlock,
            12 => BlockKind::Table,
            _ => BlockKind::Paragraph,
        }
    }
}

/// Cache-optimized Structure of Arrays (SoA) table storing parsed block records.
///
/// By splitting attributes into separate vectors, linear traversals looking for
/// specific criteria (e.g. searching block kinds or finding which block encloses an offset)
/// only stream the necessary columns through the L1/L2 data cache.
#[cfg(feature = "alloc")]
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BlockTable {
    pub kind: Vec<u8>,
    pub start: Vec<u32>,
    pub end: Vec<u32>,
    pub depth: Vec<u8>,
    pub flags: Vec<u8>,
}

#[cfg(feature = "alloc")]
impl BlockTable {
    pub fn new() -> Self {
        Self {
            kind: Vec::new(),
            start: Vec::new(),
            end: Vec::new(),
            depth: Vec::new(),
            flags: Vec::new(),
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            kind: Vec::with_capacity(capacity),
            start: Vec::with_capacity(capacity),
            end: Vec::with_capacity(capacity),
            depth: Vec::with_capacity(capacity),
            flags: Vec::with_capacity(capacity),
        }
    }

    #[inline(always)]
    pub fn len(&self) -> usize {
        self.kind.len()
    }

    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.kind.is_empty()
    }

    #[inline(always)]
    pub fn clear(&mut self) {
        self.kind.clear();
        self.start.clear();
        self.end.clear();
        self.depth.clear();
        self.flags.clear();
    }

    #[inline(always)]
    pub fn push(&mut self, kind: BlockKind, start: u32, end: u32, depth: u8, flags: u8) {
        self.kind.push(kind as u8);
        self.start.push(start);
        self.end.push(end);
        self.depth.push(depth);
        self.flags.push(flags);
    }

    /// Truncates the table to `len` blocks.
    pub fn truncate(&mut self, len: usize) {
        self.kind.truncate(len);
        self.start.truncate(len);
        self.end.truncate(len);
        self.depth.truncate(len);
        self.flags.truncate(len);
    }

    /// Finds the block index containing the given byte offset using binary search on `end`.
    pub fn find_block_at(&self, offset: u32) -> Option<usize> {
        let count = self.len();
        if count == 0 {
            return None;
        }

        let mut low = 0;
        let mut high = count;
        while low < high {
            let mid = low + (high - low) / 2;
            if self.end[mid] <= offset {
                low = mid + 1;
            } else if self.start[mid] > offset {
                high = mid;
            } else {
                return Some(mid);
            }
        }
        None
    }
}
