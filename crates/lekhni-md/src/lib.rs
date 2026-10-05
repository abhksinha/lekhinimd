#![no_std]

#[cfg(feature = "alloc")]
extern crate alloc;

pub mod block;
pub mod checkpoint;
pub mod inline;
#[cfg(feature = "alloc")]
pub mod parser;
