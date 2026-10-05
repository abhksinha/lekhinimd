#![no_std]

#[cfg(feature = "alloc")]
extern crate alloc;

pub mod buffer;
pub mod cursor;
#[cfg(feature = "alloc")]
pub mod ime;
#[cfg(feature = "alloc")]
pub mod line_index;
pub mod shaper;
#[cfg(feature = "alloc")]
pub mod undo;
