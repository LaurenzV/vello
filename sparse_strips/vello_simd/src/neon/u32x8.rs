use crate::Index;
use crate::neon::u32x4::u32x4;
use std::arch::aarch64::{uint32x4x2_t, vst1q_u32_x2};
use std::ops::Add;

#[derive(Copy, Clone, Debug)]
pub(crate) struct u32x8(pub(crate) u32x4, pub(crate) u32x4);

impl Add for u32x8 {
    type Output = Self;

    #[inline(always)]
    fn add(mut self, rhs: Self) -> Self::Output {
        self.0 = self.0 + rhs.0;
        self.1 = self.1 + rhs.1;

        self
    }
}

impl Index for u32x8 {
    type Mask = uint32x4x2_t;

    #[inline(always)]
    fn store(self, storage: &mut [u32]) {
        let storage: &mut [u32; 8] = storage.try_into().unwrap();

        unsafe { vst1q_u32_x2(storage.as_mut_ptr(), uint32x4x2_t(self.0.0, self.1.0)) }
    }

    #[inline(always)]
    fn splat(value: u32) -> Self {
        let splatted = u32x4::splat(value);
        Self(splatted, splatted)
    }

    #[inline(always)]
    fn geq(mut self, other: Self) -> Self::Mask {
        let a = self.0.geq(other.0);
        let b = self.1.geq(other.1);

        uint32x4x2_t(a, b)
    }

    #[inline(always)]
    fn if_then_else(cond: Self::Mask, if_: Self, else_: Self) -> Self {
        let a = u32x4::if_then_else(cond.0, if_.0, else_.0);
        let b = u32x4::if_then_else(cond.1, if_.1, else_.1);

        Self(a, b)
    }
}
