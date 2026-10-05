//! Cache-conscious fixed-size bitset.

/// Inline fixed-size bitset backed by an array of 64-bit words.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BitSet<const WORDS: usize> {
    words: [u64; WORDS],
}

impl<const WORDS: usize> Default for BitSet<WORDS> {
    #[inline(always)]
    fn default() -> Self {
        Self::new()
    }
}

impl<const WORDS: usize> BitSet<WORDS> {
    #[inline(always)]
    pub const fn new() -> Self {
        Self { words: [0; WORDS] }
    }

    #[inline(always)]
    pub const fn capacity_bits(&self) -> usize {
        WORDS * 64
    }

    #[inline(always)]
    pub fn contains(&self, bit: usize) -> bool {
        let word_idx = bit / 64;
        let bit_idx = bit % 64;
        if word_idx < WORDS {
            (self.words[word_idx] & (1u64 << bit_idx)) != 0
        } else {
            false
        }
    }

    #[inline(always)]
    pub fn insert(&mut self, bit: usize) -> bool {
        let word_idx = bit / 64;
        let bit_idx = bit % 64;
        if word_idx < WORDS {
            let mask = 1u64 << bit_idx;
            let existed = (self.words[word_idx] & mask) != 0;
            self.words[word_idx] |= mask;
            !existed
        } else {
            false
        }
    }

    #[inline(always)]
    pub fn remove(&mut self, bit: usize) -> bool {
        let word_idx = bit / 64;
        let bit_idx = bit % 64;
        if word_idx < WORDS {
            let mask = 1u64 << bit_idx;
            let existed = (self.words[word_idx] & mask) != 0;
            self.words[word_idx] &= !mask;
            existed
        } else {
            false
        }
    }

    #[inline(always)]
    pub fn clear(&mut self) {
        self.words = [0; WORDS];
    }
}
