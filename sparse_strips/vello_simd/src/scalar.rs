use crate::util::scalar::div_255;
use crate::{Base, Type, Scalar, Widened};
use std::ops::{Add, Mul, Sub};

impl Scalar for f32 {
    const ZERO: Self = 0.0;
    const MID: Self = 0.5;
    const ONE: Self = 1.0;

    #[inline(always)]
    fn to_rgba8(src: &[Self]) -> [u8; 4] {
        [
            (src[0] * 255.0 + 0.5) as u8,
            (src[1] * 255.0 + 0.5) as u8,
            (src[2] * 255.0 + 0.5) as u8,
            (src[3] * 255.0 + 0.5) as u8,
        ]
    }
}

impl Scalar for u8 {
    const ZERO: Self = 0;
    const MID: Self = 127;
    const ONE: Self = 255;

    #[inline(always)]
    fn to_rgba8(src: &[Self]) -> [u8; 4] {
        [src[0], src[1], src[2], src[3]]
    }
}

impl Base for f32 {}
impl Base for u8 {}
impl Base for u16 {}

#[derive(Copy, Clone, Debug)]
pub struct f32x4([f32; 4]);

impl Add for f32x4 {
    type Output = Self;

    #[inline(always)]
    fn add(mut self, rhs: Self) -> Self::Output {
        for i in 0..4 {
            self.0[i] = self.0[i] + rhs.0[i];
        }

        self
    }
}

impl Mul for f32x4 {
    type Output = Self;

    #[inline(always)]
    fn mul(mut self, rhs: Self) -> Self::Output {
        for i in 0..4 {
            self.0[i] = self.0[i] * rhs.0[i];
        }

        self
    }
}

impl Sub for f32x4 {
    type Output = Self;

    #[inline(always)]
    fn sub(mut self, rhs: Self) -> Self::Output {
        for i in 0..4 {
            self.0[i] = self.0[i] - rhs.0[i];
        }

        self
    }
}

impl Base for f32x4 {}

impl Type for f32x4 {
    type Widened = Self;

    const LENGTH: usize = 4;

    #[inline(always)]
    fn load(src: &[f32]) -> Self {
        Self((*src).try_into().unwrap())
    }

    #[inline(always)]
    fn load_4(src: &[f32; 4]) -> Self {
        Self(*src)
    }

    #[inline(always)]
    fn splat(value: f32) -> Self {
        Self([value; 4])
    }

    #[inline(always)]
    fn store(self, dest: &mut [f32]) {
        dest.copy_from_slice(&self.0)
    }

    #[inline(always)]
    fn widen(self) -> Self::Widened {
        self
    }

    #[inline(always)]
    fn normalized_mul(self, other: Self) -> Self {
        self * other
    }

    #[inline(always)]
    fn from_normalized_u8(value: u8) -> Self {
        Self::splat(value as f32 / 255.0)
    }

    type Scalar = f32;

    #[inline(always)]
    fn load_alphas(src: &[u8]) -> Self {
        Self::from_normalized_u8(src[0])
    }
}

impl Widened<f32x4> for f32x4 {
    #[inline(always)]
    fn narrow(self) -> f32x4 {
        self
    }

    #[inline(always)]
    fn normalize(self) -> Self {
        self
    }
}

#[derive(Copy, Clone, Debug)]
pub struct u16x16([u16; 16]);

impl Add for u16x16 {
    type Output = Self;

    #[inline(always)]
    fn add(mut self, rhs: Self) -> Self::Output {
        for i in 0..16 {
            self.0[i] = self.0[i] + rhs.0[i];
        }

        self
    }
}

impl Mul for u16x16 {
    type Output = Self;

    #[inline(always)]
    fn mul(mut self, rhs: Self) -> Self::Output {
        for i in 0..16 {
            self.0[i] = self.0[i] * rhs.0[i];
        }

        self
    }
}

impl Sub for u16x16 {
    type Output = Self;

    #[inline(always)]
    fn sub(mut self, rhs: Self) -> Self::Output {
        for i in 0..16 {
            self.0[i] = self.0[i] - rhs.0[i];
        }

        self
    }
}

impl Base for u16x16 {}

impl Widened<u8x16> for u16x16 {
    #[inline(always)]
    fn narrow(self) -> u8x16 {
        let mut converted = [0u8; 16];

        for i in 0..16 {
            converted[i] = self.0[i] as u8;
        }

        u8x16(converted)
    }

    #[inline(always)]
    fn normalize(mut self) -> Self {
        for i in 0..16 {
            self.0[i] = div_255(self.0[i]);
        }

        self
    }
}

#[derive(Copy, Clone, Debug)]
pub struct u8x16([u8; 16]);

impl Add for u8x16 {
    type Output = Self;

    #[inline(always)]
    fn add(mut self, rhs: Self) -> Self::Output {
        for i in 0..16 {
            self.0[i] = self.0[i] + rhs.0[i];
        }

        self
    }
}

impl Mul for u8x16 {
    type Output = Self;

    #[inline(always)]
    fn mul(mut self, rhs: Self) -> Self::Output {
        for i in 0..16 {
            self.0[i] = self.0[i] * rhs.0[i];
        }

        self
    }
}

impl Sub for u8x16 {
    type Output = Self;

    #[inline(always)]
    fn sub(mut self, rhs: Self) -> Self::Output {
        for i in 0..16 {
            self.0[i] = self.0[i] - rhs.0[i];
        }

        self
    }
}

impl Base for u8x16 {}

impl Type for u8x16 {
    type Scalar = u8;
    type Widened = u16x16;

    const LENGTH: usize = 16;

    #[inline(always)]
    fn load(src: &[u8]) -> Self {
        Self(src.try_into().unwrap())
    }

    #[inline(always)]
    fn load_alphas(src: &[u8]) -> Self {
        Self([
            src[0], src[0], src[0], src[0], src[1], src[1], src[1], src[1], src[2], src[2], src[2],
            src[2], src[3], src[3], src[3], src[3],
        ])
    }

    #[inline(always)]
    fn load_4(src: &[u8; 4]) -> Self {
        let mut result = [0u8; 16];
        
        for res in result.chunks_exact_mut(4) {
            res.copy_from_slice(src);
        }
        
        Self(result)
    }

    #[inline(always)]
    fn splat(value: u8) -> Self {
        Self([value; 16])
    }

    #[inline(always)]
    fn from_normalized_u8(value: u8) -> Self {
        Self::splat(value)
    }

    #[inline(always)]
    fn store(self, dest: &mut [u8]) {
        dest.copy_from_slice(&self.0)
    }

    #[inline(always)]
    fn widen(self) -> Self::Widened {
        let mut converted = [0u16; 16];

        for i in 0..16 {
            converted[i] = self.0[i] as u16;
        }

        u16x16(converted)
    }
}
