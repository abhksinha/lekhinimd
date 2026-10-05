//! Document metadata table (Array of Structures - AoS).
//!
//! Follows Section 4.3 of LEKHNI_ARCHITECTURE:
//! Complete record: id, path_hash, mtime, size, title.

#[cfg(feature = "alloc")]
use alloc::string::String;
#[cfg(feature = "alloc")]
use alloc::vec::Vec;

/// Complete metadata record for an indexed document.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DocRecord {
    pub id: u32,
    pub path_hash: u64,
    pub mtime: u64,
    pub size: u64,
    #[cfg(feature = "alloc")]
    pub title: String,
    #[cfg(feature = "alloc")]
    pub path: String,
}

/// Document table storing records in contiguous array layout.
#[cfg(feature = "alloc")]
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DocTable {
    pub docs: Vec<DocRecord>,
}

#[cfg(feature = "alloc")]
impl DocTable {
    pub fn new() -> Self {
        Self { docs: Vec::new() }
    }

    pub fn insert(&mut self, record: DocRecord) {
        self.docs.push(record);
    }

    pub fn get_by_id(&self, id: u32) -> Option<&DocRecord> {
        self.docs.iter().find(|d| d.id == id)
    }

    pub fn len(&self) -> usize {
        self.docs.len()
    }

    pub fn is_empty(&self) -> bool {
        self.docs.is_empty()
    }
}
