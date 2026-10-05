//! Input Method Editor (IME) state and event processing.
//!
//! Follows Section 10 of LEKHNI_ARCHITECTURE:
//! Core holds preedit string, cursor within preedit, committed text,
//! and computes caret rect back for OS candidate window placement.

#[cfg(feature = "alloc")]
use alloc::string::String;
use crate::buffer::PieceTable;
use crate::cursor::Cursor;

/// Represents an ongoing IME preedit text state.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Preedit {
    #[cfg(feature = "alloc")]
    pub text: String,
    pub cursor_byte_offset: usize,
    pub selection: Option<(usize, usize)>,
}

/// Incoming IME events from the platform shell.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ImeEvent {
    Enabled,
    #[cfg(feature = "alloc")]
    Preedit(String, Option<(usize, usize)>),
    #[cfg(feature = "alloc")]
    Commit(String),
    Disabled,
}

/// Core IME manager tracking active preedits and cursor placement.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct ImeManager {
    pub is_enabled: bool,
    pub active_preedit: Option<Preedit>,
}

impl ImeManager {
    pub const fn new() -> Self {
        Self {
            is_enabled: false,
            active_preedit: None,
        }
    }

    /// Handles an incoming IME event, modifying the piece table when text is committed.
    #[cfg(feature = "alloc")]
    pub fn handle_event<'a>(
        &mut self,
        event: ImeEvent,
        buffer: &mut PieceTable<'a>,
        cursor: &mut Cursor,
    ) {
        match event {
            ImeEvent::Enabled => {
                self.is_enabled = true;
                self.active_preedit = None;
            }
            ImeEvent::Preedit(text, selection) => {
                if text.is_empty() {
                    self.active_preedit = None;
                } else {
                    let cursor_pos = selection.map(|(s, _)| s).unwrap_or(text.len());
                    self.active_preedit = Some(Preedit {
                        text,
                        cursor_byte_offset: cursor_pos,
                        selection,
                    });
                }
            }
            ImeEvent::Commit(text) => {
                self.active_preedit = None;
                if !text.is_empty() {
                    buffer.insert(cursor.byte_offset, text.as_bytes());
                    cursor.byte_offset += text.len();
                }
            }
            ImeEvent::Disabled => {
                self.is_enabled = false;
                self.active_preedit = None;
            }
        }
    }

    /// Returns the active preedit slice if currently composing.
    #[cfg(feature = "alloc")]
    pub fn preedit_text(&self) -> Option<&str> {
        self.active_preedit.as_ref().map(|p| p.text.as_str())
    }
}
