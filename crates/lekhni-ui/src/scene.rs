//! 16-byte packed display list commands for CPU raster and GPU presentation.
//!
//! Follows Section 4.4 of LEKHNI_ARCHITECTURE:
//! Flat Vec<Cmd> reused each frame. 16-byte packed commands.

#[cfg(feature = "alloc")]
use alloc::vec::Vec;
use lekhni_raster::damage::Rect;

/// Display command category.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum CmdKind {
    FillRect = 0,
    GlyphRun = 1,
    Line = 2,
    ClipPush = 3,
    ClipPop = 4,
}

/// 16-byte packed display command. Exactly 4 commands fit in an L1 64-byte cache line.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(C)]
pub struct DisplayCmd {
    pub x: i16,
    pub y: i16,
    pub width: u16,
    pub height: u16,
    pub color: u32,
    pub kind: CmdKind,
    pub _pad: [u8; 3],
}

impl DisplayCmd {
    pub const fn fill_rect(rect: Rect, color: u32) -> Self {
        Self {
            x: rect.x as i16,
            y: rect.y as i16,
            width: rect.width as u16,
            height: rect.height as u16,
            color,
            kind: CmdKind::FillRect,
            _pad: [0; 3],
        }
    }
}

/// Retained display list with generation counter.
#[cfg(feature = "alloc")]
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DisplayList {
    pub commands: Vec<DisplayCmd>,
    pub generation: u64,
}

#[cfg(feature = "alloc")]
impl DisplayList {
    pub fn new() -> Self {
        Self {
            commands: Vec::new(),
            generation: 0,
        }
    }

    pub fn clear(&mut self) {
        self.commands.clear();
        self.generation = self.generation.wrapping_add(1);
    }

    pub fn push(&mut self, cmd: DisplayCmd) {
        self.commands.push(cmd);
    }

    pub fn len(&self) -> usize {
        self.commands.len()
    }

    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }
}
