use crate::{Numerical, Float};
use std::ops::{Add, Mul, Sub};

impl Numerical for f32 {}
impl Numerical for u8 {}
impl Numerical for u16 {}

#[derive(Copy, Clone, Debug)]
pub struct f32x4([f32; 4]);

impl Add for f32x4 {
    type Output = Self;

    #[inline(always)]
    fn add(self, rhs: Self) -> Self::Output {
        let mut result = [0.0; 4];
        for i in 0..4 {
            result[i] = self.0[i] + rhs.0[i];
        }
        Self(result)
    }
}

impl Mul for f32x4 {
    type Output = Self;

    #[inline(always)]
    fn mul(self, rhs: Self) -> Self::Output {
        let mut result = [0.0; 4];
        for i in 0..4 {
            result[i] = self.0[i] * rhs.0[i];
        }
        Self(result)
    }
}

impl Sub for f32x4 {
    type Output = Self;

    #[inline(always)]
    fn sub(self, rhs: Self) -> Self::Output {
        let mut result = [0.0; 4];
        for i in 0..4 {
            result[i] = self.0[i] - rhs.0[i];
        }
        Self(result)
    }
}

impl Numerical for f32x4 {}

impl Float<4> for f32x4 {
    #[inline(always)]
    fn splat(value: f32) -> Self {
        Self([value; 4])
    }

    #[inline(always)]
    fn load(src: &[f32; 4]) -> Self {
        Self(*src)
    }

    #[inline(always)]
    fn store(self, dest: &mut [f32; 4]) {
        dest.copy_from_slice(&self.0)
    }
}

#[derive(Copy, Clone, Debug)]
pub struct u16x8([u16; 8]);

impl Add for u16x8 {
    type Output = Self;

    #[inline(always)]
    fn add(self, rhs: Self) -> Self::Output {
        let mut result = [0; 8];
        for i in 0..8 {
            result[i] = self.0[i] + rhs.0[i];
        }
        Self(result)
    }
}

impl Mul for u16x8 {
    type Output = Self;

    #[inline(always)]
    fn mul(self, rhs: Self) -> Self::Output {
        let mut result = [0; 8];
        for i in 0..8 {
            result[i] = self.0[i] * rhs.0[i];
        }
        Self(result)
    }
}

impl Sub for u16x8 {
    type Output = Self;

    #[inline(always)]
    fn sub(self, rhs: Self) -> Self::Output {
        let mut result = [0; 8];
        for i in 0..8 {
            result[i] = self.0[i] - rhs.0[i];
        }
        Self(result)
    }
}

impl Numerical for u16x8 {}

#[derive(Copy, Clone, Debug)]
pub struct u8x16([u8; 16]);

impl Add for u8x16 {
    type Output = Self;

    #[inline(always)]
    fn add(self, rhs: Self) -> Self::Output {
        let mut result = [0; 16];
        for i in 0..16 {
            result[i] = self.0[i] + rhs.0[i];
        }
        Self(result)
    }
}

impl Mul for u8x16 {
    type Output = Self;

    #[inline(always)]
    fn mul(self, rhs: Self) -> Self::Output {
        let mut result = [0; 16];
        for i in 0..16 {
            result[i] = self.0[i] * rhs.0[i];
        }
        Self(result)
    }
}

impl Sub for u8x16 {
    type Output = Self;

    #[inline(always)]
    fn sub(self, rhs: Self) -> Self::Output {
        let mut result = [0; 16];
        for i in 0..16 {
            result[i] = self.0[i] - rhs.0[i];
        }
        Self(result)
    }
}

impl Numerical for u8x16 {}
