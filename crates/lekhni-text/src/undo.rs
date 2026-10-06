//! Undo and Redo manager storing cheap piece-descriptor snapshots.
//!
//! Because pieces are 16-byte descriptors and both original and add buffers
//! are append-only/immutable, an undo snapshot only clones the list of pieces.

#[cfg(feature = "alloc")]
use alloc::sync::Arc;
#[cfg(feature = "alloc")]
use alloc::vec::Vec;
use crate::buffer::Piece;

/// A snapshot of buffer state for undo/redo.
/// Uses reference-counted slice (`Arc<[Piece]>`) to make taking a snapshot O(1)
/// pointer clone without heap reallocations.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UndoSnapshot {
    pub pieces: Arc<[Piece]>,
    pub cursor_offset: usize,
    pub total_len: usize,
    pub total_lines: usize,
}

/// Undo/Redo stack manager.
#[derive(Default)]
pub struct UndoManager {
    undo_stack: Vec<UndoSnapshot>,
    redo_stack: Vec<UndoSnapshot>,
}

impl UndoManager {
    pub const fn new() -> Self {
        Self {
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
        }
    }

    /// Pushes an undo checkpoint before applying an edit.
    pub fn push_undo(&mut self, snapshot: UndoSnapshot) {
        self.redo_stack.clear();
        self.undo_stack.push(snapshot);
    }

    /// Pops the last undo state and pushes the current state to the redo stack.
    pub fn undo(&mut self, current: UndoSnapshot) -> Option<UndoSnapshot> {
        let prev = self.undo_stack.pop()?;
        self.redo_stack.push(current);
        Some(prev)
    }

    /// Pops the last redo state and pushes the current state to the undo stack.
    pub fn redo(&mut self, current: UndoSnapshot) -> Option<UndoSnapshot> {
        let next = self.redo_stack.pop()?;
        self.undo_stack.push(current);
        Some(next)
    }

    /// Clears all undo and redo history.
    pub fn clear(&mut self) {
        self.undo_stack.clear();
        self.redo_stack.clear();
    }
}
