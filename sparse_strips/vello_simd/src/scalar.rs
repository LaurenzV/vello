use crate::{Float, Integer, Numerical, Scalar};
use std::ops::{Add, Mul, Sub};

impl Scalar for f32 {
    const ZERO: Self = 0.0;
    const MID: Self = 0.5;
    const ONE: Self = 1.0;
}

impl Scalar for u8 {
    const ZERO: Self = 0;
    const MID: Self = 127;
    const ONE: Self = 255;
}

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
    fn store(self, dest: &mut [f32; 4]) {
        dest.copy_from_slice(&self.0)
    }

    #[inline(always)]
    fn load(src: &[f32; 4]) -> Self {
        Self(*src)
    }

    #[inline(always)]
    fn load_4(src: &[f32; 4]) -> Self {
        Self(*src)
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

impl Integer<16> for u8x16 {
    #[inline(always)]
    fn splat(value: u8) -> Self {
        Self([value; 16])
    }

    #[inline(always)]
    fn load(src: &[u8; 16]) -> Self {
        Self(*src)
    }

    #[inline(always)]
    fn load_4(src: &[u8; 4]) -> Self {
        let mut result = [0u8; 16];
        result[..4].copy_from_slice(src);
        Self(result)
    }

    #[inline(always)]
    fn store(self, dest: &mut [u8; 16]) {
        dest.copy_from_slice(&self.0)
    }
}
