use crate::neon::u8x16::{u8x16, u16x16};
use crate::{Widened, arith_ops};
use std::arch::aarch64::*;

#[derive(Copy, Clone, Debug)]
pub(crate) struct u8x32(pub(crate) u8x16, pub(crate) u8x16);

arith_ops!(u8x32);

impl u8x32 {
    #[inline(always)]
    fn load(src: &[u8]) -> Self {
        let src: &[u8; 32] = src.try_into().unwrap();

        unsafe {
            let loaded = vld1q_u8_x2(src.as_ptr());

            Self(u8x16(loaded.0), u8x16(loaded.1))
        }
    }

    #[inline(always)]
    pub(crate) fn splat_4(src: [u8; 4]) -> Self {
        unsafe {
            let loaded = u8x16::splat_4(src);

            Self(loaded, loaded)
        }
    }

    #[inline(always)]
    pub(crate) fn splat(value: u8) -> Self {
        unsafe {
            let loaded = u8x16::splat(value);

            Self(loaded, loaded)
        }
    }

    #[inline(always)]
    fn from_normalized_u8(value: u8) -> Self {
        Self::splat(value)
    }

    #[inline(always)]
    pub(crate) fn widen(self) -> u16x32 {
        let first = self.0.widen();
        let second = self.1.widen();

        u16x32(first, second)
    }

    #[inline(always)]
    pub(crate) fn normalized_mul(self, other: Self) -> u16x32 {
        let first = self.0.normalized_widening_mul(other.0);
        let second = self.1.normalized_widening_mul(other.1);

        u16x32(first, second)
    }

    #[inline(always)]
    pub fn min(mut self, other: Self) -> Self {
        self.0 = self.0.min(other.0);
        self.1 = self.1.min(other.1);

        self
    }

    #[inline(always)]
    pub fn max(mut self, other: Self) -> Self {
        self.0 = self.0.max(other.0);
        self.1 = self.1.max(other.1);

        self
    }

    #[inline(always)]
    pub(crate) fn splat_4th_element(mut self) -> Self {
        self.0 = self.0.splat_4th_element();
        self.1 = self.1.splat_4th_element();

        self
    }
}

#[derive(Copy, Clone, Debug)]
pub(crate) struct u16x32(pub(crate) u16x16, pub(crate) u16x16);

arith_ops!(u16x32);

impl u16x32 {
    #[inline(always)]
    pub(crate) fn narrow(self) -> u8x32 {
        let first = self.0.narrow();
        let second = self.1.narrow();

        u8x32(first, second)
    }

    #[inline(always)]
    pub(crate) fn normalize(mut self) -> Self {
        self.0 = self.0.normalize();
        self.1 = self.1.normalize();

        self
    }

    #[inline(always)]
    pub(crate) fn clamp(mut self) -> Self {
        self.0 = self.0.clamp();
        self.1 = self.1.clamp();

        self
    }
}
