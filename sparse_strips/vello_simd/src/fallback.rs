// Copyright 2025 the Vello Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use crate::{Base, ColorLike, Convertible, Float, NumberKind, Simd, Type, Widened};
use std::ops::{Add, Div, Mul, Sub};

#[derive(Copy, Clone, Debug)]
pub struct Fallback;

impl Simd for Fallback {
    type Integer = u8x16;
    type Float = f32x4;
}

impl NumberKind for f32 {
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

impl NumberKind for u8 {
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

impl Div for f32x4 {
    type Output = Self;

    #[inline(always)]
    fn div(mut self, rhs: Self) -> Self::Output {
        for i in 0..4 {
            self.0[i] = self.0[i] / rhs.0[i];
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
        let src = [src[0], src[1], src[2], src[3]];
        Self(src)
    }

    #[inline(always)]
    fn splat_4(src: [f32; 4]) -> Self {
        Self(src)
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
    fn normalized_mul_add(self, other1: Self, other2: Self) -> Self {
        self * other1 + other2
    }

    #[inline(always)]
    fn normalized_mul_mul_add(self, other1: Self, other2: Self, other3: Self) -> Self {
        self * other1 + other2 * other3
    }

    #[inline(always)]
    fn normalized_mul_sub(self, other1: Self, other2: Self) -> Self {
        other2 - self * other1
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

    #[inline(always)]
    fn splat_color<T: ColorLike>(color: T) -> Self {
        Self::splat_4(color.to_rgbf32())
    }

    #[inline(always)]
    fn splat_alpha<T: ColorLike>(color: T) -> Self {
        Self::splat(color.to_rgbf32()[3])
    }

    type Float = Self;
}

impl Convertible<f32x4> for f32x4 {
    #[inline(always)]
    fn convert(val: &[f32]) -> Self {
        Self::load(val)
    }
}

impl Convertible<u8x16> for f32x4 {
    #[inline(always)]
    fn convert(val: &[u8]) -> Self {
        f32x4([
            val[0] as f32 / 255.0,
            val[1] as f32 / 255.0,
            val[2] as f32 / 255.0,
            val[3] as f32 / 255.0,
        ])
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

impl Float for f32x4 {
    #[inline(always)]
    fn sqrt(self) -> Self {
        f32x4([
            self.0[0].sqrt(),
            self.0[1].sqrt(),
            self.0[2].sqrt(),
            self.0[3].sqrt(),
        ])
    }

    #[inline(always)]
    fn powf(self, exponent: f32) -> Self {
        f32x4([
            self.0[0].powf(exponent),
            self.0[1].powf(exponent),
            self.0[2].powf(exponent),
            self.0[3].powf(exponent),
        ])
    }

    #[inline(always)]
    fn splat_col_pos(pos: (f32, f32), _: (f32, f32), y_advance: (f32, f32)) -> (Self, Self) {
        let x_pos = f32x4([
            pos.0,
            pos.0 + y_advance.0,
            pos.0 + y_advance.0 * 2.0,
            pos.0 + y_advance.0 * 3.0,
        ]);
        
        let y_pos = f32x4([
            pos.1,
            pos.1 + y_advance.1,
            pos.1 + y_advance.1 * 2.0,
            pos.1 + y_advance.1 * 3.0,
        ]);

        (x_pos, y_pos)
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
    type Float = f32x4;

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
    fn splat_4(src: [u8; 4]) -> Self {
        let mut result = [0u8; 16];

        for res in result.chunks_exact_mut(4) {
            res.copy_from_slice(&src);
        }

        Self(result)
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

/// Perform an approximate division by 255.
///
/// There are three reasons for having this method.
/// 1) Divisions are slower than shifting + adding, and the compiler does not seem to replace
///    divisions by 255 with an equivalent (this was verified by benchmarking; doing / 255 was
///    significantly slower).
/// 2) Integer divisions are usually not available in SIMD, so this provides a good baseline
///    implementation.
/// 3) There are two options for performing the division: One is to perform the division
///    in a way that completely preserves the rounding semantics of a integer division by
///    255. This could be achieved using the implementation `(val + 1 + (val >> 8)) >> 8`.
///    The second approach (used here) has slightly different rounding behavior to a
///    normal division by 255, but is much faster (see <https://github.com/linebender/vello/issues/904>)
///    and therefore preferable for the high-performance pipeline.
///
/// Four properties worth mentioning:
/// - This actually calculates the ceiling of `val / 256`.
/// - Within the allowed range for `val`, rounding errors do not appear for values divisible by 255, i.e. any call `div_255(val * 255)` will always yield `val`.
/// - If there is a discrepancy, this division will always yield a value 1 higher than the original.
/// - This holds for values of `val` up to and including `65279`. You should not call this function with higher values.
#[inline(always)]
pub(crate) const fn div_255(val: u16) -> u16 {
    debug_assert!(
        val < 65280,
        "the properties of `div_255` do not hold for values of `65280` or greater"
    );
    (val + 255) >> 8
}

#[cfg(test)]
mod tests {
    use crate::fallback::div_255;

    #[test]
    fn division() {
        for i in 0_u16..=(255 * 255) {
            let expected = i / 255;
            let actual = div_255(i);

            let diff = expected.abs_diff(actual);

            // Rounding error shouldn't be higher than 1.
            assert!(diff <= 1);

            if i % 255 == 0 {
                // Division should be accurate for multiples of 255.
                assert_eq!(diff, 0);
            }
        }
    }
}
