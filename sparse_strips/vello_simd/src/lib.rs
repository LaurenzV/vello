// Copyright 2025 the Vello Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

#![allow(non_camel_case_types)]
#![allow(missing_docs)]
#![expect(
    non_camel_case_types,
    reason = "We want our SIMD types to not necessarily be camel case."
)]

pub mod fallback;
mod macros;
#[cfg(target_arch = "aarch64")]
pub mod neon;

use std::fmt::Debug;
use std::ops::{Add, Div, Mul, Sub};

/// A SIMD level for a specific target architecture.
pub trait Simd: Copy + Debug + Sized {
    type Integer: Type;
    type Float: Type;
}

pub trait Base:
    Sized
    + Copy
    + Add<Self, Output = Self>
    + Mul<Self, Output = Self>
    + Sub<Self, Output = Self>
    + Debug
{
}

// Unfortunately we cannot make C an associated constant instead, because generic const expressions
// are unstable and therefore we can't use them in `load` and `store`.
// We also need to explicitly define `A` for the same reason.
pub trait Type: Base {
    type Scalar: NumberKind;
    type Widened: Widened<Self>;
    type Float: Float;

    const IS_FLOAT: bool;
    const LENGTH: usize;

    fn load(src: &[Self::Scalar]) -> Self;
    fn load_alphas(src: &[u8]) -> Self;
    fn load_alphas_f32(src: &[f32]) -> Self;
    fn load_f32_many(src: &[f32]) -> Self;
    fn splat_4(src: [Self::Scalar; 4]) -> Self;
    fn splat_4th_element(self) -> Self;
    fn splat_color<T: ColorLike>(color: T) -> Self;
    fn splat_alpha<T: ColorLike>(color: T) -> Self;
    fn splat(value: Self::Scalar) -> Self;
    fn min(self, other: Self) -> Self;
    fn max(self, other: Self) -> Self;
    fn from_normalized_u8(value: u8) -> Self;
    fn store(self, dest: &mut [Self::Scalar]);
    fn widen(self) -> Self::Widened;
    fn from_float(f: &[Self::Float]) -> Self;

    #[inline(always)]
    fn zero() -> Self {
        Self::splat(NumberKind::ZERO)
    }

    #[inline(always)]
    fn mid() -> Self {
        Self::splat(NumberKind::MID)
    }

    #[inline(always)]
    fn one() -> Self {
        Self::splat(NumberKind::ONE)
    }

    #[inline(always)]
    fn one_minus(self) -> Self {
        Self::one() - self
    }

    #[inline(always)]
    fn normalized_mul(self, other: Self) -> Self {
        (self.widen() * other.widen()).normalize().narrow()
    }

    #[inline(always)]
    fn normalized_mul_add(self, other1: Self, other2: Self) -> Self {
        self.normalized_mul(other1) + other2
    }

    #[inline(always)]
    fn normalized_mul_mul_add(self, other1: Self, other2: Self, other3: Self) -> Self {
        let p1 = self.widen() * other1.widen();
        let p2 = other2.widen() * other3.widen();

        (p1 + p2).normalize().narrow()
    }

    #[inline(always)]
    fn normalized_mul_sub(self, other1: Self, other2: Self) -> Self {
        other2 - self.normalized_mul(other1)
    }

    #[inline(always)]
    fn pack(
        out_buf: &mut [u8],
        in_buf: &mut [Self::Scalar],
        x: usize,
        y: usize,
        width: usize,
        height: usize,
    ) {
        pack::<Self>(out_buf, in_buf, x, y, width, height);
    }
}

pub trait NumberKind: Base {
    const ZERO: Self;
    const MID: Self;
    const ONE: Self;

    fn to_rgba8(src: &[Self]) -> [u8; 4];
    fn from_normalized_f32(val: f32) -> Self;
    fn normalized_mul(&self, other: Self) -> Self;
}

pub trait Widened<N: Type>: Base {
    fn narrow(self) -> N;
    fn normalize(self) -> Self;
    fn clamp(self) -> Self;
}

pub trait ColorLike: Copy + Debug {
    fn to_rgba8(self) -> [u8; 4];
    fn to_rgbf32(self) -> [f32; 4];
}

pub trait Float: Type<Scalar = f32, Float = Self> + Div<Self, Output = Self> {
    type Mask: Mask;

    fn sqrt(self) -> Self;
    fn powf(self, exponent: Self::Scalar) -> Self;
    fn abs(self) -> Self;
    fn floor(self) -> Self;
    fn fract(self) -> Self;
    fn lt(self, other: Self) -> Self::Mask;
    fn leq(self, other: Self) -> Self::Mask;
    fn ne(self, other: Self) -> Self::Mask;

    fn if_then_else(cond: Self::Mask, if_: Self, else_: Self) -> Self;

    #[inline(always)]
    fn mul_add(self, other1: Self, other2: Self) -> Self {
        Self::normalized_mul_add(self, other1, other2)
    }

    #[inline(always)]
    fn mul_sub(self, other1: Self, other2: Self) -> Self {
        Self::normalized_mul_sub(self, other1, other2)
    }

    // See https://raphlinus.github.io/audio/2018/09/05/sigmoid.html for a little
    // explanation of this approximation to the erf function.
    // Doing `inline(always)` seems to reduce performance for some reason.
    /// Approximate the erf function.
    fn compute_erf7(x: Self) -> Self {
        let x = x * Self::splat(core::f32::consts::FRAC_2_SQRT_PI);
        let xx = x * x;
        let p1 = xx.mul_add(Self::splat(0.0104), Self::splat(0.03395));
        let p2 = xx.mul_add(p1, Self::splat(0.24295));
        let p3 = x * xx;
        let x = p3.mul_add(p2, x);
        let denom = x.mul_add(x, Self::splat(1.0)).sqrt();
        x / denom
    }

    fn splat_col_pos(
        base_pos: (f32, f32),
        x_advance: (f32, f32),
        y_advance: (f32, f32),
    ) -> (Self, Self);
}

pub trait Index: Add<Self, Output = Self> + Copy + Debug + Sized {
    type Mask: Mask;

    fn store(self, storage: &mut [u32]);
    fn splat(value: u32) -> Self;
    fn geq(self, other: Self) -> Self::Mask;

    fn if_then_else(cond: Self::Mask, if_: Self, else_: Self) -> Self;
}

pub trait Mask: Sized {
    fn splat(value: bool) -> Self;
}

pub(crate) const TILE_HEIGHT: usize = 4;
pub(crate) const WIDE_TILE_WIDTH: usize = 256;
pub(crate) const COLOR_COMPONENTS: usize = 4;

#[inline(always)]
pub(crate) fn pack<F: Type>(
    out_buf: &mut [u8],
    in_buf: &mut [F::Scalar],
    x: usize,
    y: usize,
    width: usize,
    height: usize,
) {
    // Make sure we don't process rows outside the range of the pixmap.
    let max_height = (height - y * TILE_HEIGHT).min(TILE_HEIGHT);

    // Make sure we don't process columns outside the range of the pixmap.
    let max_width = (width - x * WIDE_TILE_WIDTH).min(WIDE_TILE_WIDTH);

    let base_ix = (y * TILE_HEIGHT * width + x * WIDE_TILE_WIDTH) * COLOR_COMPONENTS;

    for j in 0..max_height {
        let line_ix = base_ix + j * width * COLOR_COMPONENTS;

        let target_len = max_width * COLOR_COMPONENTS;
        // This helps the compiler to understand that any access to `dest` cannot
        // be out of bounds, and thus saves corresponding checks in the for loop.
        let dest = &mut out_buf[line_ix..][..target_len];

        for i in 0..max_width {
            let src = &in_buf[(i * TILE_HEIGHT + j) * COLOR_COMPONENTS..][..COLOR_COMPONENTS];
            dest[i * COLOR_COMPONENTS..][..COLOR_COMPONENTS]
                .copy_from_slice(&F::Scalar::to_rgba8(src));
        }
    }
}
