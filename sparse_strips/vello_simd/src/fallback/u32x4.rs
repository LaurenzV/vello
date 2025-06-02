use crate::Index;
use crate::fallback::f32x4::f32x4;
use std::ops::Add;

#[derive(Copy, Clone, Debug)]
pub struct u32x4(pub(crate) [u32; 4]);

impl Add for u32x4 {
    type Output = Self;

    #[inline(always)]
    fn add(self, rhs: Self) -> Self::Output {
        Self([
            self.0[0] + rhs.0[0],
            self.0[1] + rhs.0[1],
            self.0[2] + rhs.0[2],
            self.0[3] + rhs.0[3],
        ])
    }
}

impl Index<f32x4> for u32x4 {
    #[inline(always)]
    fn store(self, storage: &mut [u32]) {
        storage.copy_from_slice(&self.0);
    }

    #[inline(always)]
    fn splat(value: u32) -> Self {
        Self([value; 4])
    }

    #[inline(always)]
    fn geq(self, other: Self) -> Self {
        u32x4([
            (self.0[0] >= other.0[0]) as u32,
            (self.0[1] >= other.0[1]) as u32,
            (self.0[2] >= other.0[2]) as u32,
            (self.0[3] >= other.0[3]) as u32,
        ])
    }

    #[inline(always)]
    fn if_then_else(cond: Self, if_: Self, else_: Self) -> Self {
        Self([
            if cond.0[0] > 0 { if_.0[0] } else { else_.0[0] },
            if cond.0[1] > 0 { if_.0[1] } else { else_.0[1] },
            if cond.0[2] > 0 { if_.0[2] } else { else_.0[2] },
            if cond.0[3] > 0 { if_.0[3] } else { else_.0[3] },
        ])
    }
}
