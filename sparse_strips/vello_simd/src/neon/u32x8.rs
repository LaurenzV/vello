use crate::Index;
use crate::neon::f32x8::f32x8;
use crate::neon::u32x4::u32x4;
use std::arch::aarch64::{uint32x4x2_t, vreinterpretq_f32_u32, vst1q_u32_x2};
use std::ops::Add;
use crate::neon::f32x4::f32x4;

#[derive(Copy, Clone, Debug)]
pub struct u32x8(pub(crate) u32x4, pub(crate) u32x4);

impl Add for u32x8 {
    type Output = Self;

    #[inline(always)]
    fn add(mut self, rhs: Self) -> Self::Output {
        self.0 = self.0 + rhs.0;
        self.1 = self.1 + rhs.1;

        self
    }
}

impl Index<f32x8> for u32x8 {
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
    fn geq(mut self, other: Self) -> Self {
        let a = self.0.geq(other.0);
        let b = self.1.geq(other.1);

        Self(a, b)
    }

    #[inline(always)]
    fn wrapping_sub(self, other: Self) -> Self {
        let a = self.0.wrapping_sub(other.0);
        let b = self.1.wrapping_sub(other.1);

        Self(a, b)
    }

    #[inline(always)]
    fn reinterpret(self) -> f32x8 {
        let a = self.0.reinterpret();
        let b = self.1.reinterpret();

        f32x8(a, b)
    }
    
    #[inline(always)]
    fn if_then_else(cond: Self, if_: Self, else_: Self) -> Self {
        let a = u32x4::if_then_else(cond.0, if_.0, else_.0);
        let b = u32x4::if_then_else(cond.1, if_.1, else_.1);

        Self(a, b)
    }
}
