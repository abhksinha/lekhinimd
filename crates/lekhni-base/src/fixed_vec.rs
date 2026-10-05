//! Fixed-capacity inline vector designed for zero-heap, L1/L2 cache friendliness.

use core::mem::MaybeUninit;
use core::ops::{Deref, DerefMut};

/// A fixed-capacity array-backed vector with no heap allocation.
pub struct FixedVec<T, const CAP: usize> {
    data: [MaybeUninit<T>; CAP],
    len: usize,
}

impl<T, const CAP: usize> Default for FixedVec<T, CAP> {
    #[inline(always)]
    fn default() -> Self {
        Self::new()
    }
}

impl<T, const CAP: usize> FixedVec<T, CAP> {
    #[inline(always)]
    pub const fn new() -> Self {
        Self {
            // SAFETY: An uninitialized array of MaybeUninit is valid.
            data: unsafe { MaybeUninit::uninit().assume_init() },
            len: 0,
        }
    }

    #[inline(always)]
    pub const fn len(&self) -> usize {
        self.len
    }

    #[inline(always)]
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    #[inline(always)]
    pub const fn capacity(&self) -> usize {
        CAP
    }

    #[inline(always)]
    pub fn push(&mut self, value: T) -> Result<(), T> {
        if self.len >= CAP {
            return Err(value);
        }
        self.data[self.len].write(value);
        self.len += 1;
        Ok(())
    }

    #[inline(always)]
    pub fn pop(&mut self) -> Option<T> {
        if self.len == 0 {
            None
        } else {
            self.len -= 1;
            // SAFETY: Element at self.len was initialized.
            Some(unsafe { self.data[self.len].assume_init_read() })
        }
    }

    #[inline(always)]
    pub fn clear(&mut self) {
        while self.pop().is_some() {}
    }

    #[inline(always)]
    pub fn as_slice(&self) -> &[T] {
        // SAFETY: data[0..len] are initialized.
        unsafe { core::slice::from_raw_parts(self.data.as_ptr() as *const T, self.len) }
    }

    #[inline(always)]
    pub fn as_mut_slice(&mut self) -> &mut [T] {
        // SAFETY: data[0..len] are initialized.
        unsafe { core::slice::from_raw_parts_mut(self.data.as_mut_ptr() as *mut T, self.len) }
    }
}

impl<T, const CAP: usize> Deref for FixedVec<T, CAP> {
    type Target = [T];

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        self.as_slice()
    }
}

impl<T, const CAP: usize> DerefMut for FixedVec<T, CAP> {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_mut_slice()
    }
}

impl<T, const CAP: usize> Drop for FixedVec<T, CAP> {
    fn drop(&mut self) {
        self.clear();
    }
}
