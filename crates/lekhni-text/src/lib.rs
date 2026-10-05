#![no_std]

#[cfg(feature = "alloc")]
extern crate alloc;

pub mod buffer;
pub mod cursor;
#[cfg(feature = "alloc")]
pub mod line_index;
#[cfg(feature = "alloc")]
pub mod undo;
