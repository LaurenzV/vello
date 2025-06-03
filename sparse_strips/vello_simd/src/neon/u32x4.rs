use crate::neon::f32x4::f32x4;
use crate::{Index, Type};
use std::arch::aarch64::*;
use std::ops::{Add, Sub};

#[derive(Copy, Clone, Debug)]
pub struct u32x4(pub(crate) uint32x4_t);

impl Add for u32x4 {
    type Output = Self;

    #[inline(always)]
    fn add(self, rhs: Self) -> Self::Output {
        unsafe { Self(vaddq_u32(self.0, rhs.0)) }
    }
}

impl Sub for u32x4 {
    type Output = Self;

    #[inline(always)]
    fn sub(self, rhs: Self) -> Self::Output {
        unsafe { Self(vsubq_u32(self.0, rhs.0)) }
    }
}

impl Index<f32x4> for u32x4 {
    #[inline(always)]
    fn store(self, storage: &mut [u32]) {
        let storage: &mut [u32; 4] = storage.try_into().unwrap();
        unsafe { vst1q_u32(storage.as_mut_ptr(), self.0) }
    }

    #[inline(always)]
    fn splat(value: u32) -> Self {
        unsafe { Self(vdupq_n_u32(value)) }
    }

    #[inline(always)]
    fn geq(self, other: Self) -> Self {
        Self(unsafe { vcgeq_u32(self.0, other.0) })
    }

    #[inline(always)]
    fn if_then_else(cond: Self, if_: Self, else_: Self) -> Self {
        unsafe { Self(vbslq_u32(cond.0, if_.0, else_.0)) }
    }

    #[inline(always)]
    fn wrapping_sub(self, other: Self) -> Self {
        self - other
    }

    #[inline(always)]
    fn reinterpret(self) -> f32x4 {
        unsafe { f32x4(vreinterpretq_f32_u32(self.0)) }
    }
}
