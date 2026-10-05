//! Fixed-point arithmetic for layout and rasterization without floating point churn.

/// 26.6 fixed point number (1/64th pixel precision), standard for font layout and rasterization.
#[derive(Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Fixed26_6(pub i32);

impl Fixed26_6 {
    pub const ZERO: Self = Self(0);
    pub const ONE: Self = Self(64);

    #[inline(always)]
    pub const fn from_int(v: i32) -> Self {
        Self(v << 6)
    }

    #[inline(always)]
    pub const fn to_int_round(self) -> i32 {
        (self.0 + 32) >> 6
    }

    #[inline(always)]
    pub const fn to_int_floor(self) -> i32 {
        self.0 >> 6
    }

    #[inline(always)]
    pub const fn to_int_ceil(self) -> i32 {
        (self.0 + 63) >> 6
    }

    #[inline(always)]
    pub const fn add(self, other: Self) -> Self {
        Self(self.0 + other.0)
    }

    #[inline(always)]
    pub const fn sub(self, other: Self) -> Self {
        Self(self.0 - other.0)
    }
}
