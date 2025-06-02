use crate::fallback::u32x8::u32x8;
use crate::{Float, Index, Mask};
use std::ops::Add;

#[derive(Copy, Clone, Debug)]
pub struct u32x16(u32x8, u32x8);

impl Add for u32x16 {
    type Output = Self;

    #[inline(always)]
    fn add(mut self, rhs: Self) -> Self::Output {
        self.0 = self.0 + rhs.0;
        self.1 = self.1 + rhs.1;

        self
    }
}

impl Index for u32x16 {
    type Mask = [<u32x8 as Index>::Mask; 2];

    #[inline(always)]
    fn store(self, storage: &mut [u32]) {
        self.0.store(&mut storage[..8]);
        self.1.store(&mut storage[8..16]);
    }

    #[inline(always)]
    fn splat(value: u32) -> Self {
        let splatted = u32x8::splat(value);
        Self(splatted, splatted)
    }

    #[inline(always)]
    fn geq(mut self, other: Self) -> Self::Mask {
        let a = self.0.geq(other.0);
        let b = self.1.geq(other.1);

        [a, b]
    }

    #[inline(always)]
    fn if_then_else(cond: Self::Mask, if_: Self, else_: Self) -> Self {
        let a = u32x8::if_then_else(cond[0], if_.0, else_.0);
        let b = u32x8::if_then_else(cond[1], if_.1, else_.1);

        Self(a, b)
    }
}
