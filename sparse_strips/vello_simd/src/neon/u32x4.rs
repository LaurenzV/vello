use std::arch::aarch64::*;
use std::ops::Add;
use crate::{Index, Type};

#[derive(Copy, Clone, Debug)]
pub(crate) struct u32x4(pub(crate) uint32x4_t);

impl Add for u32x4 {
    type Output = Self;

    #[inline(always)]
    fn add(self, rhs: Self) -> Self::Output {
        unsafe { Self(vaddq_u32(self.0, rhs.0)) }
    }
}

impl Index for u32x4 {
    type Mask = uint32x4_t;

    #[inline(always)]
    fn store(self, storage: &mut [u32]) {
        let storage: &mut [u32; 4] = storage.try_into().unwrap();
        unsafe {
            vst1q_u32(storage.as_mut_ptr(), self.0)
        }
    }

    #[inline(always)]
    fn splat(value: u32) -> Self {
        unsafe {
            Self(vdupq_n_u32(value))
        }
    }

    #[inline(always)]
    fn geq(self, other: Self) -> Self::Mask {
        unsafe {
            vcgeq_u32(self.0, other.0)
        }
    }

    #[inline(always)]
    fn if_then_else(cond: Self::Mask, if_: Self, else_: Self) -> Self {
        unsafe { Self(vbslq_u32(cond, if_.0, else_.0)) }
    }
}