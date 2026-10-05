//! Fast non-cryptographic hashing for caches and content identifiers.

/// 64-bit FNV-1a hash, fast, branchless, zero-state.
#[inline(always)]
pub const fn fnv1a_64(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf29ce484222325u64;
    let mut i = 0;
    while i < bytes.len() {
        hash ^= bytes[i] as u64;
        hash = hash.wrapping_mul(0x100000001b3);
        i += 1;
    }
    hash
}
