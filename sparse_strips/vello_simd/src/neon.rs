// Copyright 2025 the Vello Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use crate::{
    Base, COLOR_COMPONENTS, ColorLike, NumberKind, Simd, TILE_HEIGHT, Type, WIDE_TILE_WIDTH,
    Widened, arith_ops,
};
use bytemuck::cast_slice;
use std::arch::aarch64::*;
use std::arch::is_aarch64_feature_detected;
use std::ops::{Add, Mul, Sub};

#[derive(Copy, Clone, Debug)]
pub struct Neon(NeonImpl);

impl Neon {
    pub fn new() -> Option<Self> {
        if is_aarch64_feature_detected!("neon") {
            Some(Self(NeonImpl))
        } else {
            None
        }
    }

    pub fn get(&self) -> impl Simd + Sized + use<> {
        self.0
    }
}

#[derive(Copy, Clone, Debug)]
struct NeonImpl;

impl Simd for NeonImpl {
    // (Note that the below comments apply to tests performed on Apple Silicon, things might be different
    // for other hardware architectures.)
    // Turns out that for u8, using uint8_16_4t is much faster when loading/storing than
    // just using uint8_16t, so we use 512-bit SIMD as our baseline for NEON.
    type Integer = u8x64;
    // For f32, the story seems to be slightly different: There is a 2x slowdown when using float32x4_t instead
    // of float32x4x2_t, but using float32x4x4_t doesn't seem to give any performance benefits. For some
    // reason 256-bit also gives quite a bit better results for rendering of alpha fills in `Fine`.
    // Because of this, we use that type as the basis.
    type Float = f32x8;
}

#[derive(Copy, Clone, Debug)]
struct f32x4(float32x4_t);

impl Add for f32x4 {
    type Output = Self;

    #[inline(always)]
    fn add(self, rhs: Self) -> Self::Output {
        unsafe { Self(vaddq_f32(self.0, rhs.0)) }
    }
}

impl Mul for f32x4 {
    type Output = Self;

    #[inline(always)]
    fn mul(self, rhs: Self) -> Self::Output {
        unsafe { Self(vmulq_f32(self.0, rhs.0)) }
    }
}

impl Sub for f32x4 {
    type Output = Self;

    #[inline(always)]
    fn sub(self, rhs: Self) -> Self::Output {
        unsafe { Self(vsubq_f32(self.0, rhs.0)) }
    }
}

impl f32x4 {
    #[inline(always)]
    fn load_alphas(src: &[u8]) -> Self {
        Self::from_normalized_u8(src[0])
    }

    #[inline(always)]
    fn splat_4(src: &[f32; 4]) -> Self {
        unsafe { Self(vld1q_f32(src.as_ptr())) }
    }

    #[inline(always)]
    fn splat(value: f32) -> Self {
        unsafe { Self(vdupq_n_f32(value)) }
    }

    #[inline(always)]
    fn from_normalized_u8(value: u8) -> Self {
        Self::splat(value as f32 / 255.0)
    }

    #[inline(always)]
    fn normalized_mul(self, other: Self) -> Self {
        self * other
    }

    #[inline(always)]
    fn normalized_mul_add(self, other1: Self, other2: Self) -> Self {
        unsafe { Self(vfmaq_f32(other2.0, self.0, other1.0)) }
    }

    #[inline(always)]
    fn normalized_mul_mul_add(self, other1: Self, other2: Self, other3: Self) -> Self {
        self.normalized_mul_add(other1, other2 * other3)
    }

    #[inline(always)]
    fn normalized_mul_sub(self, other1: Self, other2: Self) -> Self {
        unsafe { Self(vfmsq_f32(other2.0, self.0, other1.0)) }
    }
}

#[derive(Copy, Clone, Debug)]
pub struct f32x8(f32x4, f32x4);

arith_ops!(f32x8);

impl Base for f32x8 {}

impl Type for f32x8 {
    type Scalar = f32;
    type Widened = Self;

    const LENGTH: usize = 8;

    #[inline(always)]
    fn load(src: &[f32]) -> Self {
        let src: &[f32; Self::LENGTH] = src.try_into().unwrap();

        unsafe {
            let loaded = vld1q_f32_x2(src.as_ptr());

            Self(f32x4(loaded.0), f32x4(loaded.1))
        }
    }

    #[inline(always)]
    fn load_alphas(src: &[u8]) -> Self {
        Self(
            f32x4::from_normalized_u8(src[0]),
            f32x4::from_normalized_u8(src[1]),
        )
    }

    #[inline(always)]
    fn splat_4(src: [f32; 4]) -> Self {
        unsafe {
            let v = vld1q_f32(src.as_ptr());

            Self(f32x4(v), f32x4(v))
        }
    }

    #[inline(always)]
    fn splat(value: f32) -> Self {
        unsafe {
            let v = vdupq_n_f32(value);

            Self(f32x4(v), f32x4(v))
        }
    }

    #[inline(always)]
    fn from_normalized_u8(value: u8) -> Self {
        Self::splat(value as f32 / 255.0)
    }

    #[inline(always)]
    fn store(self, dest: &mut [f32]) {
        let dest: &mut [f32; Self::LENGTH] = dest.try_into().unwrap();

        let stored = float32x4x2_t(self.0.0, self.1.0);
        unsafe { vst1q_f32_x2(dest.as_mut_ptr(), stored) }
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
        Self(
            self.0.normalized_mul_add(other1.0, other2.0),
            self.1.normalized_mul_add(other1.1, other2.1),
        )
    }

    #[inline(always)]
    fn normalized_mul_sub(self, other1: Self, other2: Self) -> Self {
        Self(
            self.0.normalized_mul_sub(other1.0, other2.0),
            self.1.normalized_mul_sub(other1.1, other2.1),
        )
    }

    #[inline(always)]
    fn normalized_mul_mul_add(self, other1: Self, other2: Self, other3: Self) -> Self {
        Self(
            self.0.normalized_mul_mul_add(other1.0, other2.0, other3.0),
            self.1.normalized_mul_mul_add(other1.1, other2.1, other3.1),
        )
    }

    #[inline(always)]
    fn splat_color<T: ColorLike>(color: T) -> Self {
        Self::splat_4(color.to_rgbf32())
    }

    #[inline(always)]
    fn splat_alpha<T: ColorLike>(color: T) -> Self {
        Self::splat(color.to_rgbf32()[3])
    }
}

impl Widened<f32x8> for f32x8 {
    #[inline(always)]
    fn narrow(self) -> f32x8 {
        self
    }

    #[inline(always)]
    fn normalize(self) -> Self {
        self
    }
}

#[derive(Copy, Clone, Debug)]
struct u16x16(uint16x8x2_t);

impl Add for u16x16 {
    type Output = Self;

    #[inline(always)]
    fn add(mut self, rhs: Self) -> Self::Output {
        unsafe {
            self.0.0 = vaddq_u16(self.0.0, rhs.0.0);
            self.0.1 = vaddq_u16(self.0.1, rhs.0.1);

            self
        }
    }
}

impl Mul for u16x16 {
    type Output = Self;

    #[inline(always)]
    fn mul(mut self, rhs: Self) -> Self::Output {
        unsafe {
            self.0.0 = vmulq_u16(self.0.0, rhs.0.0);
            self.0.1 = vmulq_u16(self.0.1, rhs.0.1);

            self
        }
    }
}

impl Sub for u16x16 {
    type Output = Self;

    #[inline(always)]
    fn sub(mut self, rhs: Self) -> Self::Output {
        unsafe {
            self.0.0 = vsubq_u16(self.0.0, rhs.0.0);
            self.0.1 = vsubq_u16(self.0.1, rhs.0.1);

            self
        }
    }
}

impl u16x16 {
    #[inline(always)]
    fn narrow(self) -> u8x16 {
        unsafe {
            let low = vmovn_u16(self.0.0);
            let high = vmovn_u16(self.0.1);

            u8x16(vcombine_u8(low, high))
        }
    }

    #[inline(always)]
    fn normalize(self) -> Self {
        Self(uint16x8x2_t(div_255(self.0.0), div_255(self.0.1)))
    }
}

#[derive(Copy, Clone, Debug)]
struct u16x32(u16x16, u16x16);

arith_ops!(u16x32);

impl u16x32 {
    #[inline(always)]
    fn narrow(self) -> u8x32 {
        let first = self.0.narrow();
        let second = self.1.narrow();

        u8x32(first, second)
    }

    #[inline(always)]
    fn normalize(mut self) -> Self {
        self.0 = self.0.normalize();
        self.1 = self.1.normalize();

        self
    }
}

#[derive(Copy, Clone, Debug)]
pub struct u16x64(u16x32, u16x32);

arith_ops!(u16x64);

impl Base for u16x64 {}

impl Widened<u8x64> for u16x64 {
    #[inline(always)]
    fn narrow(self) -> u8x64 {
        let first = self.0.narrow();
        let second = self.1.narrow();

        u8x64(first, second)
    }

    #[inline(always)]
    fn normalize(mut self) -> Self {
        self.0 = self.0.normalize();
        self.1 = self.1.normalize();

        self
    }
}

#[derive(Copy, Clone, Debug)]
struct u8x16(uint8x16_t);

impl Add for u8x16 {
    type Output = Self;

    #[inline(always)]
    fn add(self, rhs: Self) -> Self::Output {
        unsafe { Self(vaddq_u8(self.0, rhs.0)) }
    }
}

impl Mul for u8x16 {
    type Output = Self;

    #[inline(always)]
    fn mul(self, rhs: Self) -> Self::Output {
        unsafe { Self(vmulq_u8(self.0, rhs.0)) }
    }
}

impl Sub for u8x16 {
    type Output = Self;

    #[inline(always)]
    fn sub(self, rhs: Self) -> Self::Output {
        unsafe { Self(vsubq_u8(self.0, rhs.0)) }
    }
}

impl u8x16 {
    #[inline(always)]
    fn load(src: &[u8]) -> Self {
        let src: &[u8; 16] = src.try_into().unwrap();

        unsafe { Self(vld1q_u8(src.as_ptr())) }
    }

    #[inline(always)]
    fn splat_4(src: [u8; 4]) -> Self {
        unsafe {
            let loaded = vreinterpretq_u8_u32(vdupq_n_u32(u32::from_ne_bytes(src)));
            Self(loaded)
        }
    }

    #[inline(always)]
    fn splat(value: u8) -> Self {
        unsafe { Self(vdupq_n_u8(value)) }
    }

    #[inline(always)]
    fn from_normalized_u8(value: u8) -> Self {
        Self::splat(value)
    }

    #[inline(always)]
    fn widen(self) -> u16x16 {
        unsafe {
            let low = vget_low_u8(self.0);
            let high = vget_high_u8(self.0);

            u16x16(uint16x8x2_t(vmovl_u8(low), vmovl_u8(high)))
        }
    }

    #[inline(always)]
    fn widening_mul(self, other: Self) -> u16x16 {
        unsafe {
            let left_low = vget_low_u8(self.0);
            let right_low = vget_low_u8(other.0);
            let high = vmull_high_u8(self.0, other.0);
            let low = vmull_u8(left_low, right_low);

            u16x16(uint16x8x2_t(low, high))
        }
    }

    #[inline(always)]
    fn normalized_widening_mul(self, other: Self) -> u16x16 {
        unsafe {
            let mut mulled = self.widening_mul(other);
            mulled.0.0 = div_255(mulled.0.0);
            mulled.0.1 = div_255(mulled.0.1);

            mulled
        }
    }
}

#[derive(Copy, Clone, Debug)]
struct u8x32(u8x16, u8x16);

arith_ops!(u8x32);

impl u8x32 {
    #[inline(always)]
    fn load(src: &[u8]) -> Self {
        let src: &[u8; 32] = src.try_into().unwrap();

        unsafe {
            let loaded = vld1q_u8_x2(src.as_ptr());

            Self(u8x16(loaded.0), u8x16(loaded.1))
        }
    }

    #[inline(always)]
    fn splat_4(src: [u8; 4]) -> Self {
        unsafe {
            let loaded = u8x16::splat_4(src);

            Self(loaded, loaded)
        }
    }

    #[inline(always)]
    fn splat(value: u8) -> Self {
        unsafe {
            let loaded = u8x16::splat(value);

            Self(loaded, loaded)
        }
    }

    #[inline(always)]
    fn from_normalized_u8(value: u8) -> Self {
        Self::splat(value)
    }

    #[inline(always)]
    fn widen(self) -> u16x32 {
        let first = self.0.widen();
        let second = self.1.widen();

        u16x32(first, second)
    }

    #[inline(always)]
    fn normalized_mul(self, other: Self) -> u16x32 {
        let first = self.0.normalized_widening_mul(other.0);
        let second = self.1.normalized_widening_mul(other.1);

        u16x32(first, second)
    }
}

#[derive(Copy, Clone, Debug)]
pub struct u8x64(u8x32, u8x32);

arith_ops!(u8x64);

impl Base for u8x64 {}

impl Type for u8x64 {
    type Scalar = u8;
    type Widened = u16x64;

    const LENGTH: usize = 64;

    #[inline(always)]
    fn load(src: &[u8]) -> Self {
        let src: &[u8; Self::LENGTH] = src.try_into().unwrap();

        unsafe {
            let loaded = vld1q_u8_x4(src.as_ptr());

            Self(
                u8x32(u8x16(loaded.0), u8x16(loaded.1)),
                u8x32(u8x16(loaded.2), u8x16(loaded.3)),
            )
        }
    }

    #[inline(always)]
    fn load_alphas(src: &[u8]) -> Self {
        let src: &[u8; Self::LENGTH / 4] = src.try_into().unwrap();

        unsafe {
            let loaded = vld1q_u8(src.as_ptr());
            let zipped = vzipq_u8(loaded, loaded);
            let zip1 = vzipq_u8(zipped.0, zipped.0);
            let zip2 = vzipq_u8(zipped.1, zipped.1);

            Self(
                u8x32(u8x16(zip1.0), u8x16(zip1.1)),
                u8x32(u8x16(zip2.0), u8x16(zip2.1)),
            )
        }
    }

    #[inline(always)]
    fn splat_4(src: [u8; 4]) -> Self {
        unsafe {
            let loaded = u8x32::splat_4(src);

            Self(loaded, loaded)
        }
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
        unsafe {
            let loaded = u8x32::splat(value);

            Self(loaded, loaded)
        }
    }

    #[inline(always)]
    fn from_normalized_u8(value: u8) -> Self {
        Self::splat(value)
    }

    #[inline(always)]
    fn store(self, dest: &mut [u8]) {
        let dest: &mut [u8; Self::LENGTH] = dest.try_into().unwrap();

        let stored = uint8x16x4_t(self.0.0.0, self.0.1.0, self.1.0.0, self.1.1.0);
        unsafe { vst1q_u8_x4(dest.as_mut_ptr(), stored) }
    }

    #[inline(always)]
    fn widen(self) -> Self::Widened {
        let first = self.0.widen();
        let second = self.1.widen();

        u16x64(first, second)
    }

    #[inline(always)]
    fn normalized_mul(self, other: Self) -> Self {
        let first = self.0.normalized_mul(other.0);
        let second = self.1.normalized_mul(other.1);

        u16x64(first, second).narrow()
    }

    #[inline(always)]
    fn normalized_mul_add(self, other1: Self, other2: Self) -> Self {
        self.normalized_mul(other1) + other2
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
        let max_height = (height - y * TILE_HEIGHT).min(TILE_HEIGHT);
        let max_width = (width - x * WIDE_TILE_WIDTH).min(WIDE_TILE_WIDTH);

        if max_height != TILE_HEIGHT || max_width != WIDE_TILE_WIDTH {
            // In theory, it would be possible to handle tiles where the pixmap does not
            // have the full height (i.e. at the very bottom) using the below code. However,
            // I'm seeing a significant slowdown in benchmarks when adding a `.take(max_height)`
            // to the iterators below, so we instead just fallback to scalar packing for those
            // edge cases, so we have the full performance for the general case.
            crate::pack::<Self>(out_buf, in_buf, x, y, width, height);
        } else {
            let (user_x, _) = (x * WIDE_TILE_WIDTH, y * TILE_HEIGHT);
            let row_len = width * COLOR_COMPONENTS;
            let mut base_slice = {
                let row_ix = y * usize::from(TILE_HEIGHT) * row_len;
                let (_, tail) = out_buf.split_at_mut(row_ix);
                tail
            };
            
            let mut dest_slices: [&mut [u8]; TILE_HEIGHT] = [&mut [], &mut [], &mut [], &mut []];
            
            for s in &mut dest_slices.iter_mut() {
                let (row, tail) = base_slice.split_at_mut(row_len);

                *s = &mut row[user_x * COLOR_COMPONENTS..][..max_width * COLOR_COMPONENTS];

                base_slice = tail;
            }

            for (idx, col) in in_buf.chunks_exact(Self::LENGTH).enumerate() {
                let dest_idx = idx * Self::LENGTH / 4;

                let casted: &[u32; 16] = cast_slice::<u8, u32>(col).try_into().unwrap();
                unsafe {
                    let loaded = vld4q_u32(casted.as_ptr());
                    let reinterpreted = [
                        vreinterpretq_u8_u32(loaded.0),
                        vreinterpretq_u8_u32(loaded.1),
                        vreinterpretq_u8_u32(loaded.2),
                        vreinterpretq_u8_u32(loaded.3),
                    ];

                    // Same as above, only take `max_height` at most since for the bottom-most wide tile,
                    // we are not guaranteed to have 4 rows in the pixmap.
                    for (dest, src) in dest_slices.iter_mut().zip(reinterpreted) {
                        let target: &mut [u8; 16] = (&mut dest[dest_idx..][..16]).try_into().unwrap();
                        vst1q_u8(target.as_mut_ptr(), src)
                    }
                }
            }
        }
    }
}

#[inline(always)]
fn div_255(input: uint16x8_t) -> uint16x8_t {
    unsafe {
        let p1 = vdupq_n_u16(255);
        let p2 = vaddq_u16(input, p1);
        vshrq_n_u16::<8>(p2)
    }
}
