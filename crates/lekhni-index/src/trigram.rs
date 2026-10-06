//! Trigram postings index for fast candidate pre-filtering.
//!
//! Follows Section 4.3 & 12 of LEKHNI_ARCHITECTURE:
//! Trigram postings: sorted u32 doc IDs for set intersection.

#[cfg(feature = "alloc")]
use alloc::vec::Vec;

/// Packs 3 ASCII/UTF-8 bytes into a 24-bit integer trigram key.
#[inline(always)]
pub const fn pack_trigram(b0: u8, b1: u8, b2: u8) -> u32 {
    ((b0 as u32) << 16) | ((b1 as u32) << 8) | (b2 as u32)
}

/// Trigram entry with associated document IDs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TrigramPosting {
    pub trigram: u32,
    #[cfg(feature = "alloc")]
    pub doc_ids: Vec<u32>,
}

/// Trigram inverted index.
#[cfg(feature = "alloc")]
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TrigramIndex {
    pub postings: Vec<TrigramPosting>,
}

#[cfg(feature = "alloc")]
impl TrigramIndex {
    pub fn new() -> Self {
        Self { postings: Vec::new() }
    }

    /// Indexes a document's text content.
    pub fn index_doc(&mut self, doc_id: u32, text: &[u8]) {
        if text.len() < 3 {
            return;
        }

        for window in text.windows(3) {
            let key = pack_trigram(
                window[0].to_ascii_lowercase(),
                window[1].to_ascii_lowercase(),
                window[2].to_ascii_lowercase(),
            );

            match self.postings.binary_search_by_key(&key, |p| p.trigram) {
                Ok(pos) => {
                    let doc_ids = &mut self.postings[pos].doc_ids;
                    if doc_ids.last().copied() != Some(doc_id) {
                        doc_ids.push(doc_id);
                    }
                }
                Err(pos) => {
                    self.postings.insert(pos, TrigramPosting {
                        trigram: key,
                        doc_ids: alloc::vec![doc_id],
                    });
                }
            }
        }
    }

    /// Finds candidate document IDs containing all trigrams in `query`.
    pub fn query_candidates(&self, query: &[u8]) -> Vec<u32> {
        if query.len() < 3 {
            return Vec::new();
        }

        let mut candidate_sets: Vec<&[u32]> = Vec::new();

        for window in query.windows(3) {
            let key = pack_trigram(
                window[0].to_ascii_lowercase(),
                window[1].to_ascii_lowercase(),
                window[2].to_ascii_lowercase(),
            );

            if let Ok(pos) = self.postings.binary_search_by_key(&key, |p| p.trigram) {
                candidate_sets.push(&self.postings[pos].doc_ids);
            } else {
                // Trigram not present in any document -> zero candidates
                return Vec::new();
            }
        }

        if candidate_sets.is_empty() {
            return Vec::new();
        }

        // Intersect candidate document ID slices using binary search on sorted doc_ids
        let mut result: Vec<u32> = candidate_sets[0].to_vec();
        for set in &candidate_sets[1..] {
            result.retain(|id| set.binary_search(id).is_ok());
        }

        result
    }
}
