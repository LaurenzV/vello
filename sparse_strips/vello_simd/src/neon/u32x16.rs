use std::arch::aarch64::*;
use std::ops::Add;
use crate::Index;
use crate::neon::u32x4::u32x4;
use crate::neon::u32x8::u32x8;

#[derive(Copy, Clone, Debug)]
pub(crate) struct u32x16(u32x8, u32x8);

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
    type Mask = uint32x4x4_t;

    #[inline(always)]
    fn store(self, storage: &mut [u32]) {
        let storage: &mut [u32; 16] = storage.try_into().unwrap();
        
        unsafe {
            vst1q_u32_x4(storage.as_mut_ptr(), uint32x4x4_t(self.0.0.0, self.0.1.0, self.1.0.0, self.1.1.0))
        }
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

        uint32x4x4_t(a.0, a.1, b.0, b.1)
    }

    #[inline(always)]
    fn if_then_else(cond: Self::Mask, if_: Self, else_: Self) -> Self {
        let a = u32x8::if_then_else(uint32x4x2_t(cond.0, cond.1), if_.0, else_.0);
        let b = u32x8::if_then_else(uint32x4x2_t(cond.2, cond.3), if_.1, else_.1);

        Self(a, b)
    }
}