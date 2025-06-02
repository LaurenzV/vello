use std::arch::aarch64::{uint32x4_t, vaddq_u32, vbslq_u32, vcgeq_u32, vdupq_n_u32, vst1q_u32};
use std::ops::Add;
use crate::Index;

#[derive(Copy, Clone, Debug)]
pub(crate) struct u32x4([u32; 4]);

impl Add for u32x4 {
    type Output = Self;

    #[inline(always)]
    fn add(self, rhs: Self) -> Self::Output {
        Self(
            [
                self.0[0] + rhs.0[0],
                self.0[1] + rhs.0[1],
                self.0[2] + rhs.0[2],
                self.0[3] + rhs.0[3]
            ]
        )
    }
}

impl Index for u32x4 {
    type Mask = [bool; 4];

    #[inline(always)]
    fn store(self, storage: &mut [u32]) {
        storage.copy_from_slice(&self.0);
    }

    #[inline(always)]
    fn splat(value: u32) -> Self {
        Self([value; 4])
    }

    #[inline(always)]
    fn geq(self, other: Self) -> Self::Mask {
        [
            self.0[0] >= other.0[0],
            self.0[1] >= other.0[1],
            self.0[2] >= other.0[2],
            self.0[3] >= other.0[3],
        ]
    }

    #[inline(always)]
    fn if_then_else(cond: Self::Mask, if_: Self, else_: Self) -> Self {
        Self(
            [
                if cond[0] { if_.0[0] } else { else_.0[0] },
                if cond[0] { if_.0[1] } else { else_.0[1] },
                if cond[0] { if_.0[2] } else { else_.0[2] },
                if cond[0] { if_.0[3] } else { else_.0[3] },
            ]
        )
    }
}