//! Checkpoints for incremental parsing.
//!
//! Follows Section 4.2 of LEKHNI_ARCHITECTURE:
//! Packed parser state in one u64 (fence open, fence char/len, list stack depth, quote depth, html-block flag).

#[cfg(feature = "alloc")]
use alloc::vec::Vec;

/// Packed parser state in 64 bits.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct PackedParserState(pub u64);

impl PackedParserState {
    pub const fn empty() -> Self {
        Self(0)
    }

    #[inline(always)]
    pub fn new(
        fence_open: bool,
        fence_char: u8,
        fence_len: u8,
        list_depth: u8,
        quote_depth: u8,
        html_block: bool,
    ) -> Self {
        let mut bits = 0u64;
        if fence_open {
            bits |= 1u64;
        }
        bits |= (fence_char as u64) << 1;
        bits |= (fence_len as u64) << 9;
        bits |= (list_depth as u64) << 17;
        bits |= (quote_depth as u64) << 25;
        if html_block {
            bits |= 1u64 << 33;
        }
        Self(bits)
    }

    #[inline(always)]
    pub fn fence_open(&self) -> bool {
        (self.0 & 1) != 0
    }

    #[inline(always)]
    pub fn fence_char(&self) -> u8 {
        ((self.0 >> 1) & 0xFF) as u8
    }

    #[inline(always)]
    pub fn fence_len(&self) -> u8 {
        ((self.0 >> 9) & 0xFF) as u8
    }

    #[inline(always)]
    pub fn list_depth(&self) -> u8 {
        ((self.0 >> 17) & 0xFF) as u8
    }

    #[inline(always)]
    pub fn quote_depth(&self) -> u8 {
        ((self.0 >> 25) & 0xFF) as u8
    }

    #[inline(always)]
    pub fn html_block(&self) -> bool {
        ((self.0 >> 33) & 1) != 0
    }
}

/// A checkpoint record recording parser state at a given line and byte boundary.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Checkpoint {
    pub line: usize,
    pub byte_offset: usize,
    pub block_index: usize,
    pub state: PackedParserState,
}

/// Checkpoint table managing periodic parser states every N lines.
#[cfg(feature = "alloc")]
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CheckpointTable {
    pub checkpoints: Vec<Checkpoint>,
    pub interval_lines: usize,
}

#[cfg(feature = "alloc")]
impl CheckpointTable {
    pub const DEFAULT_INTERVAL: usize = 64;

    pub fn new() -> Self {
        Self {
            checkpoints: Vec::new(),
            interval_lines: Self::DEFAULT_INTERVAL,
        }
    }

    pub fn with_interval(interval: usize) -> Self {
        Self {
            checkpoints: Vec::new(),
            interval_lines: interval,
        }
    }

    #[inline(always)]
    pub fn clear(&mut self) {
        self.checkpoints.clear();
    }

    #[inline(always)]
    pub fn push(&mut self, cp: Checkpoint) {
        self.checkpoints.push(cp);
    }

    /// Finds the nearest checkpoint at or before `line`.
    pub fn find_checkpoint_for_line(&self, line: usize) -> Option<Checkpoint> {
        let mut best: Option<Checkpoint> = None;
        for cp in &self.checkpoints {
            if cp.line <= line {
                best = Some(*cp);
            } else {
                break;
            }
        }
        best
    }

    /// Truncates checkpoints starting from the given line.
    pub fn truncate_from_line(&mut self, line: usize) {
        self.checkpoints.retain(|cp| cp.line < line);
    }
}
