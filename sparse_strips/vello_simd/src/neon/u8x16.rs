use crate::neon::div_255;
use std::arch::aarch64::*;
use std::ops::{Add, Mul, Sub};

#[derive(Copy, Clone, Debug)]
pub(crate) struct u8x16(pub(crate) uint8x16_t);

impl Add for u8x16 {
    type Output = Self;

    #[inline(always)]
    fn add(self, rhs: Self) -> Self::Output {
        unsafe { Self(vaddq_u8(self.0, rhs.0)) }
    }
}

impl Mul for u8x16 {
    type Output = Self;

    #[inline(always)]
    fn mul(self, rhs: Self) -> Self::Output {
        unsafe { Self(vmulq_u8(self.0, rhs.0)) }
    }
}

impl Sub for u8x16 {
    type Output = Self;

    #[inline(always)]
    fn sub(self, rhs: Self) -> Self::Output {
        unsafe { Self(vsubq_u8(self.0, rhs.0)) }
    }
}

impl u8x16 {
    #[inline(always)]
    fn load(src: &[u8]) -> Self {
        let src: &[u8; 16] = src.try_into().unwrap();

        unsafe { Self(vld1q_u8(src.as_ptr())) }
    }

    #[inline(always)]
    pub(crate) fn splat_4(src: [u8; 4]) -> Self {
        unsafe {
            let loaded = vreinterpretq_u8_u32(vdupq_n_u32(u32::from_ne_bytes(src)));
            Self(loaded)
        }
    }

    #[inline(always)]
    pub(crate) fn splat(value: u8) -> Self {
        unsafe { Self(vdupq_n_u8(value)) }
    }

    #[inline(always)]
    fn from_normalized_u8(value: u8) -> Self {
        Self::splat(value)
    }

    #[inline(always)]
    pub(crate) fn widen(self) -> u16x16 {
        unsafe {
            let low = vget_low_u8(self.0);
            let high = vget_high_u8(self.0);

            u16x16(uint16x8x2_t(vmovl_u8(low), vmovl_u8(high)))
        }
    }

    #[inline(always)]
    fn widening_mul(self, other: Self) -> u16x16 {
        unsafe {
            let left_low = vget_low_u8(self.0);
            let right_low = vget_low_u8(other.0);
            let high = vmull_high_u8(self.0, other.0);
            let low = vmull_u8(left_low, right_low);

            u16x16(uint16x8x2_t(low, high))
        }
    }

    #[inline(always)]
    pub(crate) fn normalized_widening_mul(self, other: Self) -> u16x16 {
        unsafe {
            let mut mulled = self.widening_mul(other);
            mulled.0.0 = div_255(mulled.0.0);
            mulled.0.1 = div_255(mulled.0.1);

            mulled
        }
    }

    #[inline(always)]
    pub fn min(self, other: Self) -> Self {
        unsafe { Self(vminq_u8(self.0, other.0)) }
    }

    #[inline(always)]
    pub fn max(self, other: Self) -> Self {
        unsafe { Self(vmaxq_u8(self.0, other.0)) }
    }

    #[inline(always)]
    pub(crate) fn splat_4th_element(self) -> Self {
        unsafe {
            let low = vget_low_u8(self.0);
            let high = vget_high_u8(self.0);

            let table = uint8x8x2_t(low, high);

            let idx_lo = vld1_u8([3, 3, 3, 3, 7, 7, 7, 7].as_ptr());
            let idx_hi = vld1_u8([11, 11, 11, 11, 15, 15, 15, 15].as_ptr());

            let out_lo = vtbl2_u8(table, idx_lo);
            let out_hi = vtbl2_u8(table, idx_hi);

            Self(vcombine_u8(out_lo, out_hi))
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub(crate) struct u16x16(pub(crate) uint16x8x2_t);

impl Add for u16x16 {
    type Output = Self;

    #[inline(always)]
    fn add(mut self, rhs: Self) -> Self::Output {
        unsafe {
            self.0.0 = vaddq_u16(self.0.0, rhs.0.0);
            self.0.1 = vaddq_u16(self.0.1, rhs.0.1);

            self
        }
    }
}

impl Mul for u16x16 {
    type Output = Self;

    #[inline(always)]
    fn mul(mut self, rhs: Self) -> Self::Output {
        unsafe {
            self.0.0 = vmulq_u16(self.0.0, rhs.0.0);
            self.0.1 = vmulq_u16(self.0.1, rhs.0.1);

            self
        }
    }
}

impl Sub for u16x16 {
    type Output = Self;

    #[inline(always)]
    fn sub(mut self, rhs: Self) -> Self::Output {
        unsafe {
            self.0.0 = vsubq_u16(self.0.0, rhs.0.0);
            self.0.1 = vsubq_u16(self.0.1, rhs.0.1);

            self
        }
    }
}

impl u16x16 {
    #[inline(always)]
    pub(crate) fn narrow(self) -> u8x16 {
        unsafe {
            let low = vmovn_u16(self.0.0);
            let high = vmovn_u16(self.0.1);

            u8x16(vcombine_u8(low, high))
        }
    }

    #[inline(always)]
    pub(crate) fn normalize(self) -> Self {
        Self(uint16x8x2_t(div_255(self.0.0), div_255(self.0.1)))
    }
}
