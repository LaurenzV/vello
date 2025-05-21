use crate::fallback::f32x4::f32x4;
use crate::fallback::u8x32::{u8x32, u16x32};
use crate::{Base, ColorLike, Type, Widened};
use std::ops::{Add, Mul, Sub};

#[derive(Copy, Clone, Debug)]
pub struct u8x64(u8x32, u8x32);

impl Add for u8x64 {
    type Output = Self;

    #[inline(always)]
    fn add(mut self, rhs: Self) -> Self::Output {
        self.0 = self.0.add(rhs.0);
        self.1 = self.1.add(rhs.1);

        self
    }
}

impl Mul for u8x64 {
    type Output = Self;

    #[inline(always)]
    fn mul(mut self, rhs: Self) -> Self::Output {
        self.0 = self.0.mul(rhs.0);
        self.1 = self.1.mul(rhs.1);

        self
    }
}

impl Sub for u8x64 {
    type Output = Self;

    #[inline(always)]
    fn sub(mut self, rhs: Self) -> Self::Output {
        self.0 = self.0.sub(rhs.0);
        self.1 = self.1.sub(rhs.1);

        self
    }
}

impl Base for u8x64 {}

impl Type for u8x64 {
    type Scalar = u8;
    type Widened = u16x64;
    type Float = f32x4;
    const IS_FLOAT: bool = false;

    const LENGTH: usize = 64;

    #[inline(always)]
    fn load(src: &[u8]) -> Self {
        Self(u8x32::load(&src[0..32]), u8x32::load(&src[32..]))
    }

    #[inline(always)]
    fn load_alphas(src: &[u8]) -> Self {
        Self(
            u8x32::load_alphas(&src[0..8]),
            u8x32::load_alphas(&src[8..]),
        )
    }

    #[inline(always)]
    fn load_alphas_f32(src: &[f32]) -> Self {
        Self(
            u8x32::load_alphas_f32(&src[0..8]),
            u8x32::load_alphas_f32(&src[8..]),
        )
    }

    #[inline(always)]
    fn load_f32_many(src: &[f32]) -> Self {
        Self(
            u8x32::load_f32_many(&src[0..32]),
            u8x32::load_f32_many(&src[32..]),
        )
    }

    #[inline(always)]
    fn splat_4(src: [u8; 4]) -> Self {
        let splat = u8x32::splat_4(src);
        Self(splat, splat)
    }

    #[inline(always)]
    fn splat_4th_element(mut self) -> Self {
        self.0 = self.0.splat_4th_element();
        self.1 = self.1.splat_4th_element();

        self
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
        Self(u8x32::splat(value), u8x32::splat(value))
    }

    #[inline(always)]
    fn min(mut self, other: Self) -> Self {
        self.0 = self.0.min(other.0);
        self.1 = self.1.min(other.1);

        self
    }

    #[inline(always)]
    fn max(mut self, other: Self) -> Self {
        self.0 = self.0.max(other.0);
        self.1 = self.1.max(other.1);

        self
    }

    #[inline(always)]
    fn from_normalized_u8(value: u8) -> Self {
        Self::splat(value)
    }

    #[inline(always)]
    fn store(self, dest: &mut [u8]) {
        self.0.store(&mut dest[0..32]);
        self.1.store(&mut dest[32..64]);
    }

    #[inline(always)]
    fn widen(self) -> Self::Widened {
        u16x64(self.0.widen(), self.1.widen())
    }

    #[inline(always)]
    fn from_float(f: &[Self::Float]) -> Self {
        Self(u8x32::from_float(&f[0..2]), u8x32::from_float(&f[2..4]))
    }
}

#[derive(Copy, Clone, Debug)]
pub struct u16x64(u16x32, u16x32);

impl Base for u16x64 {}

impl Add for u16x64 {
    type Output = Self;

    #[inline(always)]
    fn add(mut self, rhs: Self) -> Self::Output {
        self.0 = self.0.add(rhs.0);
        self.1 = self.1.add(rhs.1);

        self
    }
}

impl Mul for u16x64 {
    type Output = Self;

    #[inline(always)]
    fn mul(mut self, rhs: Self) -> Self::Output {
        self.0 = self.0.mul(rhs.0);
        self.1 = self.1.mul(rhs.1);

        self
    }
}

impl Sub for u16x64 {
    type Output = Self;

    #[inline(always)]
    fn sub(mut self, rhs: Self) -> Self::Output {
        self.0 = self.0.sub(rhs.0);
        self.1 = self.1.sub(rhs.1);

        self
    }
}

impl Widened<u8x64> for u16x64 {
    #[inline(always)]
    fn narrow(self) -> u8x64 {
        u8x64(self.0.narrow(), self.1.narrow())
    }

    #[inline(always)]
    fn normalize(mut self) -> Self {
        self.0 = self.0.normalize();
        self.1 = self.1.normalize();

        self
    }
}
