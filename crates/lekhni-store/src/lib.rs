#![no_std]

#[cfg(feature = "alloc")]
extern crate alloc;

pub mod atomic_save;
pub mod external_edit;
pub mod notebook;
pub mod recovery;
