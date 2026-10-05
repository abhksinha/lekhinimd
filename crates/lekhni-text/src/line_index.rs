//! Fast line index navigation over piece table chunks.

#[cfg(feature = "alloc")]
use crate::buffer::PieceTable;

#[cfg(feature = "alloc")]
impl<'a> PieceTable<'a> {
    /// Returns the (line_index, line_start_byte_offset) for the given byte offset.
    pub fn offset_to_line(&self, target_offset: usize) -> (usize, usize) {
        if target_offset >= self.len() && !self.is_empty() {
            // End of buffer clamp
            return (self.line_breaks(), self.len());
        }

        let mut current_line = 0;
        let mut line_start_offset = 0;
        let mut curr_offset = 0;

        for piece in &self.pieces {
            let piece_len = piece.length as usize;
            if curr_offset + piece_len <= target_offset {
                current_line += piece.line_breaks as usize;
                if piece.line_breaks > 0 {
                    // Line started somewhere inside this piece, but we passed it
                }
                curr_offset += piece_len;
                continue;
            }

            // Target is inside this piece
            let piece_slice = self.piece_slice(piece);
            let inside_offset = target_offset - curr_offset;

            for (idx, &b) in piece_slice[..inside_offset].iter().enumerate() {
                if b == b'\n' {
                    current_line += 1;
                    line_start_offset = curr_offset + idx + 1;
                }
            }
            break;
        }

        (current_line, line_start_offset)
    }

    /// Returns the starting byte offset of the given 0-indexed line.
    pub fn line_to_offset(&self, target_line: usize) -> usize {
        if target_line == 0 {
            return 0;
        }
        if target_line > self.line_breaks() {
            return self.len();
        }

        let mut current_line = 0;
        let mut curr_offset = 0;

        for piece in &self.pieces {
            let piece_lines = piece.line_breaks as usize;
            if current_line + piece_lines < target_line {
                current_line += piece_lines;
                curr_offset += piece.length as usize;
                continue;
            }

            // Target line starts within this piece
            let piece_slice = self.piece_slice(piece);
            for (idx, &b) in piece_slice.iter().enumerate() {
                if b == b'\n' {
                    current_line += 1;
                    if current_line == target_line {
                        return curr_offset + idx + 1;
                    }
                }
            }
            curr_offset += piece.length as usize;
        }

        self.len()
    }
}
