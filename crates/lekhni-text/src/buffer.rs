//! Piece table text buffer implementation for Lekhni text engine.
//!
//! Follows Section 4.1 of LEKHNI_ARCHITECTURE:
//! - Immutable original buffer (e.g. mmap'd or initial text) + append-only add buffer.
//! - Pieces stored in a flat contiguous array with cache-line alignment.
//! - In-place piece coalescing on consecutive typing to eliminate allocations in the steady state.

#[cfg(feature = "alloc")]
use alloc::vec::Vec;

/// Specifies whether a piece references the original slice or the append-only add buffer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum BufferSource {
    Original = 0,
    Add = 1,
}

/// A 16-byte piece descriptor. Four piece descriptors fit exactly in a 64-byte L1 cache line.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(C)]
pub struct Piece {
    pub start: u32,
    pub length: u32,
    pub line_breaks: u32,
    pub source: BufferSource,
    _pad: [u8; 3],
}

impl Piece {
    #[inline(always)]
    pub const fn new(source: BufferSource, start: u32, length: u32, line_breaks: u32) -> Self {
        Self {
            start,
            length,
            line_breaks,
            source,
            _pad: [0; 3],
        }
    }
}

/// Count newline bytes ('\n') in a byte slice without heap allocation.
#[inline(always)]
pub fn count_newlines(bytes: &[u8]) -> u32 {
    let mut count = 0u32;
    for &b in bytes {
        if b == b'\n' {
            count += 1;
        }
    }
    count
}

/// PieceTable text buffer.
#[cfg(feature = "alloc")]
pub struct PieceTable<'a> {
    original: &'a [u8],
    add_buffer: Vec<u8>,
    pub(crate) pieces: Vec<Piece>,
    pub(crate) total_len: usize,
    pub(crate) total_lines: usize,
}

#[cfg(feature = "alloc")]
impl<'a> PieceTable<'a> {
    /// Creates a new piece table from an existing original byte slice.
    pub fn new(original: &'a [u8]) -> Self {
        let lines = count_newlines(original) as usize;
        let mut pieces = Vec::new();
        if !original.is_empty() {
            pieces.push(Piece::new(
                BufferSource::Original,
                0,
                original.len() as u32,
                lines as u32,
            ));
        }

        Self {
            original,
            add_buffer: Vec::new(),
            pieces,
            total_len: original.len(),
            total_lines: lines,
        }
    }

    /// Takes a snapshot of the current piece descriptors for undo.
    pub fn take_snapshot(&self, cursor_offset: usize) -> crate::undo::UndoSnapshot {
        crate::undo::UndoSnapshot {
            pieces: alloc::sync::Arc::from(self.pieces.as_slice()),
            cursor_offset,
            total_len: self.total_len,
            total_lines: self.total_lines,
        }
    }

    /// Restores a snapshot of piece descriptors from an undo state.
    pub fn restore_snapshot(&mut self, snapshot: crate::undo::UndoSnapshot) {
        self.pieces = snapshot.pieces.to_vec();
        self.total_len = snapshot.total_len;
        self.total_lines = snapshot.total_lines;
    }

    /// Total byte length of the text.
    #[inline(always)]
    pub fn len(&self) -> usize {
        self.total_len
    }

    /// Returns true if the buffer has zero bytes.
    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.total_len == 0
    }

    /// Total newline count in the buffer.
    #[inline(always)]
    pub fn line_breaks(&self) -> usize {
        self.total_lines
    }

    /// Resolves which slice a piece points to.
    #[inline(always)]
    pub(crate) fn piece_slice(&self, piece: &Piece) -> &[u8] {
        let start = piece.start as usize;
        let end = start + piece.length as usize;
        match piece.source {
            BufferSource::Original => &self.original[start..end],
            BufferSource::Add => &self.add_buffer[start..end],
        }
    }

    /// Reads a single byte at the given byte offset.
    pub fn byte_at(&self, mut offset: usize) -> Option<u8> {
        if offset >= self.total_len {
            return None;
        }
        for piece in &self.pieces {
            let piece_len = piece.length as usize;
            if offset < piece_len {
                let slice = self.piece_slice(piece);
                return Some(slice[offset]);
            }
            offset -= piece_len;
        }
        None
    }

    /// Inserts `text` at `offset`.
    /// Coalesces consecutive typing at the end of the last add piece in-place with zero re-allocations.
    pub fn insert(&mut self, offset: usize, text: &[u8]) {
        if text.is_empty() {
            return;
        }
        assert!(offset <= self.total_len, "Offset out of bounds");

        let add_start = self.add_buffer.len() as u32;
        self.add_buffer.extend_from_slice(text);
        let text_lines = count_newlines(text);
        let new_piece = Piece::new(
            BufferSource::Add,
            add_start,
            text.len() as u32,
            text_lines,
        );

        self.total_len += text.len();
        self.total_lines += text_lines as usize;

        // If buffer was empty
        if self.pieces.is_empty() {
            self.pieces.push(new_piece);
            return;
        }

        // Fast path: append at end with piece coalescing
        if offset == self.total_len - text.len() {
            if let Some(last) = self.pieces.last_mut() {
                if last.source == BufferSource::Add && last.start + last.length == add_start {
                    last.length += text.len() as u32;
                    last.line_breaks += text_lines;
                    return;
                }
            }
            self.pieces.push(new_piece);
            return;
        }

        // General path: locate piece to split
        let mut curr_offset = 0;
        for i in 0..self.pieces.len() {
            let piece_len = self.pieces[i].length as usize;
            if curr_offset + piece_len >= offset {
                let split_point = offset - curr_offset;

                if split_point == 0 {
                    // Check if can coalesce with previous piece
                    if i > 0 && self.pieces[i - 1].source == BufferSource::Add && self.pieces[i - 1].start + self.pieces[i - 1].length == add_start {
                        self.pieces[i - 1].length += text.len() as u32;
                        self.pieces[i - 1].line_breaks += text_lines;
                        return;
                    }
                    self.pieces.insert(i, new_piece);
                } else if split_point == piece_len {
                    // Check if can coalesce with current piece
                    if self.pieces[i].source == BufferSource::Add && self.pieces[i].start + self.pieces[i].length == add_start {
                        self.pieces[i].length += text.len() as u32;
                        self.pieces[i].line_breaks += text_lines;
                        return;
                    }
                    self.pieces.insert(i + 1, new_piece);
                } else {
                    let old_piece = self.pieces[i];
                    let left_slice = match old_piece.source {
                        BufferSource::Original => {
                            &self.original[old_piece.start as usize..old_piece.start as usize + split_point]
                        }
                        BufferSource::Add => {
                            &self.add_buffer[old_piece.start as usize..old_piece.start as usize + split_point]
                        }
                    };
                    let left_lines = count_newlines(left_slice);
                    let right_lines = old_piece.line_breaks - left_lines;

                    let left_piece = Piece::new(
                        old_piece.source,
                        old_piece.start,
                        split_point as u32,
                        left_lines,
                    );
                    let right_piece = Piece::new(
                        old_piece.source,
                        old_piece.start + split_point as u32,
                        old_piece.length - split_point as u32,
                        right_lines,
                    );

                    self.pieces[i] = left_piece;
                    self.pieces.insert(i + 1, new_piece);
                    self.pieces.insert(i + 2, right_piece);
                }
                return;
            }
            curr_offset += piece_len;
        }
    }

    /// Deletes `length` bytes starting at `offset`.
    pub fn delete(&mut self, offset: usize, length: usize) {
        if length == 0 || self.total_len == 0 {
            return;
        }
        assert!(offset + length <= self.total_len, "Range out of bounds");

        let delete_end = offset + length;
        let mut curr_offset = 0;
        let mut i = 0;

        let mut deleted_lines = 0;

        while i < self.pieces.len() {
            let piece_len = self.pieces[i].length as usize;
            let piece_start = curr_offset;
            let piece_end = curr_offset + piece_len;

            if piece_end <= offset {
                // Entirely before deletion
                curr_offset = piece_end;
                i += 1;
                continue;
            }

            if piece_start >= delete_end {
                // Entirely after deletion
                break;
            }

            // Piece intersects deletion range
            let piece = self.pieces[i];
            let overlap_start = offset.saturating_sub(piece_start);
            let overlap_end = if piece_end > delete_end {
                delete_end - piece_start
            } else {
                piece_len
            };

            // Count newlines in deleted slice
            let piece_slice = self.piece_slice(&piece);
            deleted_lines += count_newlines(&piece_slice[overlap_start..overlap_end]) as usize;

            if overlap_start == 0 && overlap_end == piece_len {
                // Entire piece deleted
                self.pieces.remove(i);
                curr_offset = piece_end;
                continue;
            } else if overlap_start == 0 {
                // Truncate start
                let remaining_slice = &piece_slice[overlap_end..];
                let rem_lines = count_newlines(remaining_slice);
                self.pieces[i].start += overlap_end as u32;
                self.pieces[i].length -= overlap_end as u32;
                self.pieces[i].line_breaks = rem_lines;
                curr_offset = piece_end;
                i += 1;
            } else if overlap_end == piece_len {
                // Truncate end
                let remaining_slice = &piece_slice[..overlap_start];
                let rem_lines = count_newlines(remaining_slice);
                self.pieces[i].length = overlap_start as u32;
                self.pieces[i].line_breaks = rem_lines;
                curr_offset = piece_end;
                i += 1;
            } else {
                // Split around deleted range
                let left_slice = &piece_slice[..overlap_start];
                let right_slice = &piece_slice[overlap_end..];
                let left_lines = count_newlines(left_slice);
                let right_lines = count_newlines(right_slice);

                let left_piece = Piece::new(
                    piece.source,
                    piece.start,
                    overlap_start as u32,
                    left_lines,
                );
                let right_piece = Piece::new(
                    piece.source,
                    piece.start + overlap_end as u32,
                    piece.length - overlap_end as u32,
                    right_lines,
                );

                self.pieces[i] = left_piece;
                self.pieces.insert(i + 1, right_piece);
                curr_offset = piece_end;
                i += 2;
            }
        }

        self.total_len -= length;
        self.total_lines -= deleted_lines;
    }

    /// Copies text into the provided output buffer and returns the number of bytes copied.
    pub fn copy_range(&self, start: usize, len: usize, dest: &mut [u8]) -> usize {
        let to_copy = core::cmp::min(len, self.total_len.saturating_sub(start));
        let to_copy = core::cmp::min(to_copy, dest.len());
        if to_copy == 0 {
            return 0;
        }

        let end = start + to_copy;
        let mut curr_offset = 0;
        let mut written = 0;

        for piece in &self.pieces {
            let piece_len = piece.length as usize;
            let piece_start = curr_offset;
            let piece_end = curr_offset + piece_len;

            if piece_end <= start {
                curr_offset = piece_end;
                continue;
            }
            if piece_start >= end {
                break;
            }

            let p_start = start.saturating_sub(piece_start);
            let p_end = if piece_end > end {
                end - piece_start
            } else {
                piece_len
            };

            let slice = &self.piece_slice(piece)[p_start..p_end];
            dest[written..written + slice.len()].copy_from_slice(slice);
            written += slice.len();
            curr_offset = piece_end;
        }

        written
    }
}
