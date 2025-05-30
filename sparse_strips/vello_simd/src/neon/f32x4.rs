use crate::Mask;
use std::arch::aarch64::*;
use std::ops::{Add, Div, Mul, Sub};

#[derive(Copy, Clone, Debug)]
pub(crate) struct f32x4(pub(crate) float32x4_t);

impl Add for f32x4 {
    type Output = Self;

    #[inline(always)]
    fn add(self, rhs: Self) -> Self::Output {
        unsafe { Self(vaddq_f32(self.0, rhs.0)) }
    }
}

impl Mul for f32x4 {
    type Output = Self;

    #[inline(always)]
    fn mul(self, rhs: Self) -> Self::Output {
        unsafe { Self(vmulq_f32(self.0, rhs.0)) }
    }
}

impl Sub for f32x4 {
    type Output = Self;

    #[inline(always)]
    fn sub(self, rhs: Self) -> Self::Output {
        unsafe { Self(vsubq_f32(self.0, rhs.0)) }
    }
}

impl Div for f32x4 {
    type Output = Self;

    #[inline(always)]
    fn div(self, rhs: Self) -> Self::Output {
        unsafe { Self(vdivq_f32(self.0, rhs.0)) }
    }
}

impl f32x4 {
    #[inline(always)]
    fn load_alphas(src: &[u8]) -> Self {
        Self::from_normalized_u8(src[0])
    }

    #[inline(always)]
    fn splat_4(src: &[f32; 4]) -> Self {
        unsafe { Self(vld1q_f32(src.as_ptr())) }
    }

    #[inline(always)]
    pub(crate) fn splat(value: f32) -> Self {
        unsafe { Self(vdupq_n_f32(value)) }
    }

    #[inline(always)]
    pub(crate) fn from_normalized_u8(value: u8) -> Self {
        Self::splat(value as f32 / 255.0)
    }

    #[inline(always)]
    fn normalized_mul(self, other: Self) -> Self {
        self * other
    }

    #[inline(always)]
    pub(crate) fn normalized_mul_add(self, other1: Self, other2: Self) -> Self {
        unsafe { Self(vfmaq_f32(other2.0, self.0, other1.0)) }
    }

    #[inline(always)]
    pub(crate) fn normalized_mul_mul_add(self, other1: Self, other2: Self, other3: Self) -> Self {
        self.normalized_mul_add(other1, other2 * other3)
    }

    #[inline(always)]
    pub(crate) fn normalized_mul_sub(self, other1: Self, other2: Self) -> Self {
        unsafe { Self(vfmsq_f32(other2.0, self.0, other1.0)) }
    }

    #[inline(always)]
    pub(crate) fn min(self, other: Self) -> Self {
        unsafe { Self(vminq_f32(self.0, other.0)) }
    }

    #[inline(always)]
    pub(crate) fn max(self, other: Self) -> Self {
        unsafe { Self(vmaxq_f32(self.0, other.0)) }
    }

    #[inline(always)]
    pub(crate) fn abs(self) -> Self {
        unsafe { Self(vabsq_f32(self.0)) }
    }

    #[inline(always)]
    pub(crate) fn floor(self) -> Self {
        unsafe { Self(vrndmq_f32(self.0)) }
    }

    #[inline(always)]
    pub(crate) fn fract(self) -> Self {
        unsafe {
            let c1 = vcvtq_s32_f32(self.0);
            let c2 = vcvtq_f32_s32(c1);

            Self(vsubq_f32(self.0, c2))
        }
    }

    #[inline(always)]
    pub(crate) fn splat_4th_element(self) -> Self {
        unsafe {
            let z0 = vzip2q_f32(self.0, self.0);
            let z1 = vzip2q_f32(z0, z0);

            Self(z1)
        }
    }

    #[inline(always)]
    pub(crate) fn lt(self, other: Self) -> uint32x4_t {
        unsafe { vcltq_f32(self.0, other.0) }
    }

    #[inline(always)]
    pub(crate) fn ne(self, other: Self) -> uint32x4_t {
        unsafe { vmvnq_u32(vceqq_f32(self.0, other.0)) }
    }

    #[inline(always)]
    pub(crate) fn if_then_else(mask: uint32x4_t, a: Self, b: Self) -> Self {
        unsafe { Self(vbslq_f32(mask, a.0, b.0)) }
    }

    #[inline(always)]
    pub(crate) fn clamp(self) -> Self {
        unsafe {
            let min = vdupq_n_f32(0.0);
            let max = vdupq_n_f32(1.0);

            Self(vmaxq_f32(vminq_f32(self.0, max), min))
        }
    }
}

impl Mask for uint32x4_t {
    #[inline(always)]
    fn splat(value: bool) -> Self {
        unsafe { vdupq_n_u32(value as u32 * u32::MAX) }
    }
}
