//! Incremental Markdown block parser.
//!
//! Follows Section 6 of LEKHNI_ARCHITECTURE:
//! - Block-level incremental reparse restarting from the nearest checkpoint <= dirty line.
//! - State convergence checking to splice cleanly into downstream checkpoints.

use crate::block::{BlockKind, BlockTable};
use crate::checkpoint::{Checkpoint, CheckpointTable, PackedParserState};

/// Incremental Markdown document parser.
#[cfg(feature = "alloc")]
pub struct MarkdownParser {
    pub blocks: BlockTable,
    pub checkpoints: CheckpointTable,
}

#[cfg(feature = "alloc")]
impl Default for MarkdownParser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "alloc")]
impl MarkdownParser {
    pub fn new() -> Self {
        Self {
            blocks: BlockTable::new(),
            checkpoints: CheckpointTable::new(),
        }
    }

    /// Performs a full parse of the document byte buffer.
    pub fn parse_full(&mut self, text: &[u8]) {
        self.blocks.clear();
        self.checkpoints.clear();
        self.parse_range(text, 0, 0, PackedParserState::empty());
    }

    /// Incrementally updates the parse tree when an edit occurs at or after `dirty_line`.
    pub fn reparse_incremental(&mut self, text: &[u8], dirty_line: usize) {
        if let Some(cp) = self.checkpoints.find_checkpoint_for_line(dirty_line) {
            // Check if there is a downstream checkpoint we can converge to
            if let Some((next_cp_idx, next_cp)) = self.checkpoints.find_next_checkpoint_after_line(dirty_line) {
                // If the downstream checkpoint offset is beyond the text length or mismatch, fall back
                if next_cp.byte_offset <= text.len() {
                    // Save downstream state to splice if parser converges at next_cp.byte_offset
                    let saved_blocks = self.blocks.clone();
                    let saved_checkpoints = self.checkpoints.checkpoints.clone();

                    self.blocks.truncate(cp.block_index);
                    if let Some(last_end) = self.blocks.end.last_mut() {
                        if *last_end > cp.byte_offset as u32 {
                            *last_end = cp.byte_offset as u32;
                        }
                    }
                    self.checkpoints.truncate_from_line(cp.line + 1);

                    let converged = self.parse_range_until(
                        text,
                        cp.line,
                        cp.byte_offset,
                        cp.state,
                        next_cp.line,
                        next_cp.state,
                    );

                    if let Some(converged_offset) = converged {
                        let delta = converged_offset as i64 - next_cp.byte_offset as i64;
                        // Splice downstream blocks shifted by delta!
                        self.blocks.extend_from_shifted(&saved_blocks, next_cp.block_index, saved_blocks.len(), delta);
                        // Restore downstream checkpoints shifted by delta
                        for mut downstream_cp in saved_checkpoints[next_cp_idx..].iter().copied() {
                            downstream_cp.byte_offset = (downstream_cp.byte_offset as i64 + delta).max(0) as usize;
                            downstream_cp.block_index = self.blocks.len() - (saved_blocks.len() - downstream_cp.block_index);
                            self.checkpoints.push(downstream_cp);
                        }
                        return;
                    }

                    // Did not converge before or at next_cp: continue parsing the rest of document
                    self.parse_range(text, next_cp.line, next_cp.byte_offset, next_cp.state);
                    return;
                }
            }

            // Roll back blocks and checkpoints after this checkpoint
            self.blocks.truncate(cp.block_index);
            if let Some(last_end) = self.blocks.end.last_mut() {
                if *last_end > cp.byte_offset as u32 {
                    *last_end = cp.byte_offset as u32;
                }
            }
            self.checkpoints.truncate_from_line(cp.line + 1);
            self.parse_range(text, cp.line, cp.byte_offset, cp.state);
        } else {
            self.parse_full(text);
        }
    }

    /// Internal loop parsing from `start_line` / `start_offset` until reaching target checkpoint line.
    /// Returns Some(converged_offset) if parser state converged with `target_state` at `target_line`.
    fn parse_range_until(
        &mut self,
        text: &[u8],
        mut line_num: usize,
        mut offset: usize,
        mut state: PackedParserState,
        target_line: usize,
        target_state: PackedParserState,
    ) -> Option<usize> {
        let total_len = text.len();

        while offset < total_len {
            if line_num == target_line {
                if state == target_state {
                    return Some(offset);
                } else {
                    return None;
                }
            }
            if line_num > target_line {
                break;
            }

            // Find end of current line
            let mut line_end = offset;
            while line_end < total_len && text[line_end] != b'\n' {
                line_end += 1;
            }

            let line_slice = &text[offset..line_end];
            let next_offset = if line_end < total_len {
                line_end + 1
            } else {
                line_end
            };

            // Periodically record checkpoints
            if line_num > 0 && line_num % self.checkpoints.interval_lines == 0 {
                let already_recorded = self.checkpoints.checkpoints.last().map(|c| c.line) == Some(line_num);
                if !already_recorded {
                    self.checkpoints.push(Checkpoint {
                        line: line_num,
                        byte_offset: offset,
                        block_index: self.blocks.len(),
                        state,
                    });
                }
            }

            self.parse_single_line(line_slice, offset, next_offset, &mut state);

            offset = next_offset;
            line_num += 1;
        }

        if line_num == target_line && state == target_state {
            Some(offset)
        } else {
            None
        }
    }

    /// Internal loop parsing from `start_line` / `start_offset` with the given initial state.
    fn parse_range(
        &mut self,
        text: &[u8],
        mut line_num: usize,
        mut offset: usize,
        mut state: PackedParserState,
    ) {
        let total_len = text.len();

        while offset < total_len {
            // Find end of current line
            let mut line_end = offset;
            while line_end < total_len && text[line_end] != b'\n' {
                line_end += 1;
            }

            // Line slice excluding newline
            let line_slice = &text[offset..line_end];
            let next_offset = if line_end < total_len {
                line_end + 1 // skip '\n'
            } else {
                line_end
            };

            // Periodically record checkpoints
            if line_num > 0 && line_num % self.checkpoints.interval_lines == 0 {
                let already_recorded = self.checkpoints.checkpoints.last().map(|c| c.line) == Some(line_num);
                if !already_recorded {
                    self.checkpoints.push(Checkpoint {
                        line: line_num,
                        byte_offset: offset,
                        block_index: self.blocks.len(),
                        state,
                    });
                }
            }

            self.parse_single_line(line_slice, offset, next_offset, &mut state);

            offset = next_offset;
            line_num += 1;
        }
    }

    /// Parses a single markdown line and appends or coalesces blocks.
    #[inline(always)]
    fn parse_single_line(
        &mut self,
        line_slice: &[u8],
        offset: usize,
        next_offset: usize,
        state: &mut PackedParserState,
    ) {
        if state.fence_open() {
            // Inside fenced code block: check for matching closing fence
            let trimmed = trim_start(line_slice);
            if let Some(last_end) = self.blocks.end.last_mut() {
                *last_end = next_offset as u32;
            }
            if is_closing_fence(trimmed, state.fence_char(), state.fence_len()) {
                *state = PackedParserState::new(false, 0, 0, state.list_depth(), state.quote_depth(), false);
            }
        } else {
            let trimmed = trim_start(line_slice);
            if trimmed.is_empty() {
                // Blank line: ends current paragraph
            } else if let Some((fence_char, fence_len)) = detect_opening_fence(trimmed) {
                *state = PackedParserState::new(true, fence_char, fence_len, state.list_depth(), state.quote_depth(), false);
                self.blocks.push(
                    BlockKind::FencedCode,
                    offset as u32,
                    next_offset as u32,
                    0,
                    0,
                );
            } else if let Some(heading_level) = detect_heading(trimmed) {
                let kind = match heading_level {
                    1 => BlockKind::Heading1,
                    2 => BlockKind::Heading2,
                    3 => BlockKind::Heading3,
                    4 => BlockKind::Heading4,
                    5 => BlockKind::Heading5,
                    _ => BlockKind::Heading6,
                };
                self.blocks.push(kind, offset as u32, next_offset as u32, 0, 0);
            } else if trimmed.starts_with(b"> ") || trimmed == b">" {
                self.blocks.push(
                    BlockKind::BlockQuote,
                    offset as u32,
                    next_offset as u32,
                    1,
                    0,
                );
            } else if is_list_item(trimmed) {
                self.blocks.push(
                    BlockKind::ListItem,
                    offset as u32,
                    next_offset as u32,
                    1,
                    0,
                );
            } else if is_thematic_break(trimmed) {
                self.blocks.push(
                    BlockKind::ThematicBreak,
                    offset as u32,
                    next_offset as u32,
                    0,
                    0,
                );
            } else if is_table_row(trimmed) {
                let can_coalesce = if let Some(&last_kind) = self.blocks.kind.last() {
                    last_kind == BlockKind::Table as u8 && self.blocks.end.last().copied() == Some(offset as u32)
                } else {
                    false
                };

                if can_coalesce {
                    if let Some(last_end) = self.blocks.end.last_mut() {
                        *last_end = next_offset as u32;
                    }
                } else {
                    self.blocks.push(
                        BlockKind::Table,
                        offset as u32,
                        next_offset as u32,
                        0,
                        0,
                    );
                }
            } else {
                // Paragraph: check if can coalesce with previous paragraph
                let can_coalesce = if let Some(&last_kind) = self.blocks.kind.last() {
                    last_kind == BlockKind::Paragraph as u8 && self.blocks.end.last().copied() == Some(offset as u32)
                } else {
                    false
                };

                if can_coalesce {
                    if let Some(last_end) = self.blocks.end.last_mut() {
                        *last_end = next_offset as u32;
                    }
                } else {
                    self.blocks.push(
                        BlockKind::Paragraph,
                        offset as u32,
                        next_offset as u32,
                        0,
                        0,
                    );
                }
            }
        }
    }
}

#[inline(always)]
fn trim_start(mut bytes: &[u8]) -> &[u8] {
    while let Some((&first, rest)) = bytes.split_first() {
        if first == b' ' || first == b'\t' {
            bytes = rest;
        } else {
            break;
        }
    }
    bytes
}

#[inline(always)]
fn detect_heading(bytes: &[u8]) -> Option<u8> {
    if bytes.starts_with(b"# ") {
        Some(1)
    } else if bytes.starts_with(b"## ") {
        Some(2)
    } else if bytes.starts_with(b"### ") {
        Some(3)
    } else if bytes.starts_with(b"#### ") {
        Some(4)
    } else if bytes.starts_with(b"##### ") {
        Some(5)
    } else if bytes.starts_with(b"###### ") {
        Some(6)
    } else {
        None
    }
}

#[inline(always)]
fn detect_opening_fence(bytes: &[u8]) -> Option<(u8, u8)> {
    if bytes.len() >= 3 && (bytes[0] == b'`' || bytes[0] == b'~') {
        let fence_char = bytes[0];
        let mut count = 0;
        for &b in bytes {
            if b == fence_char {
                count += 1;
            } else {
                break;
            }
        }
        if count >= 3 {
            return Some((fence_char, count.min(255) as u8));
        }
    }
    None
}

#[inline(always)]
fn is_closing_fence(bytes: &[u8], fence_char: u8, fence_len: u8) -> bool {
    let mut count = 0;
    for &b in bytes {
        if b == fence_char {
            count += 1;
        } else if b == b' ' || b == b'\t' {
            continue;
        } else {
            return false;
        }
    }
    count >= fence_len
}

#[inline(always)]
fn is_list_item(bytes: &[u8]) -> bool {
    if bytes.starts_with(b"- ") || bytes.starts_with(b"* ") || bytes.starts_with(b"+ ") {
        return true;
    }
    // Ordered list: 1. or 1)
    if let Some((&first, rest)) = bytes.split_first() {
        if first.is_ascii_digit() && (rest.starts_with(b". ") || rest.starts_with(b") ")) {
            return true;
        }
    }
    false
}

#[inline(always)]
fn is_thematic_break(bytes: &[u8]) -> bool {
    let mut chars = [0u8; 3];
    let mut count = 0;
    let mut mark = 0;

    for &b in bytes {
        if b == b' ' || b == b'\t' {
            continue;
        }
        if b == b'-' || b == b'*' || b == b'_' {
            if mark == 0 {
                mark = b;
            } else if mark != b {
                return false;
            }
            if count < 3 {
                chars[count] = b;
            }
            count += 1;
        } else {
            return false;
        }
    }
    count >= 3
}

#[inline(always)]
fn is_table_row(bytes: &[u8]) -> bool {
    let bar_count = bytes.iter().filter(|&&b| b == b'|').count();
    bar_count >= 2 && (bytes.starts_with(b"|") || bytes.ends_with(b"|"))
}
