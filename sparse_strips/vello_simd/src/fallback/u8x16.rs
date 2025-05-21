use crate::fallback::div_255;
use crate::fallback::f32x4::f32x4;
use crate::{Base, ColorLike, Type, Widened};
use std::ops::{Add, Mul, Sub};

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
    type Float = f32x4;
    const IS_FLOAT: bool = false;

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
    fn load_alphas_f32(src: &[f32]) -> Self {
        let c = |v: f32| (v * 255.0 + 0.5) as u8;

        Self([
            c(src[0]),
            c(src[0]),
            c(src[0]),
            c(src[0]),
            c(src[1]),
            c(src[1]),
            c(src[1]),
            c(src[1]),
            c(src[2]),
            c(src[2]),
            c(src[2]),
            c(src[2]),
            c(src[3]),
            c(src[3]),
            c(src[3]),
            c(src[3]),
        ])
    }

    #[inline(always)]
    fn load_f32_many(src: &[f32]) -> Self {
        let src: &[f32; 16] = src.try_into().unwrap();
        let mut storage = [0u8; 16];

        for i in 0..16 {
            storage[i] = (src[i] * 255.0 + 0.5) as u8;
        }

        Self(storage)
    }

    #[inline(always)]
    fn splat_4(src: [u8; 4]) -> Self {
        let mut result = [0u8; 16];

        for res in result.chunks_exact_mut(4) {
            res.copy_from_slice(&src);
        }

        Self(result)
    }

    #[inline(always)]
    fn splat_4th_element(self) -> Self {
        Self([
            self.0[3], self.0[3], self.0[3], self.0[3], self.0[7], self.0[7], self.0[7], self.0[7],
            self.0[11], self.0[11], self.0[11], self.0[11], self.0[15], self.0[15], self.0[15],
            self.0[15],
        ])
    }

    #[inline(always)]
    fn splat_color<T: ColorLike>(color: T) -> Self {
        Self::splat_4(color.to_rgba8())
    }

    #[inline(always)]
    fn splat_alpha<T: ColorLike>(color: T) -> Self {
        Self::splat(color.to_rgba8()[3])
    }

    #[inline(always)]
    fn splat(value: u8) -> Self {
        Self([value; 16])
    }

    #[inline(always)]
    fn min(mut self, other: Self) -> Self {
        for i in 0..16 {
            self.0[i] = self.0[i].min(other.0[i]);
        }

        self
    }

    #[inline(always)]
    fn max(mut self, other: Self) -> Self {
        for i in 0..16 {
            self.0[i] = self.0[i].max(other.0[i]);
        }

        self
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

    #[inline(always)]
    fn from_float(f: &[Self::Float]) -> Self {
        let f: &[f32x4; 1] = f.try_into().unwrap();
        let f = f[0].0;
        let mut storage = [0u8; 16];

        for (s, f) in storage.chunks_exact_mut(4).zip(f) {
            s[0] = (f * 255.0 + 0.5) as u8;
            s[1] = (f * 255.0 + 0.5) as u8;
            s[2] = (f * 255.0 + 0.5) as u8;
            s[3] = (f * 255.0 + 0.5) as u8;
        }

        Self(storage)
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
