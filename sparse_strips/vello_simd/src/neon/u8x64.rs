use crate::neon::f32x8::f32x8;
use crate::neon::u8x16::u8x16;
use crate::neon::u8x32::{u8x32, u16x32};
use crate::{
    Base, COLOR_COMPONENTS, ColorLike, TILE_HEIGHT, Type, WIDE_TILE_WIDTH, Widened, arith_ops,
};
use bytemuck::cast_slice;
use std::arch::aarch64::*;
use crate::neon::f32x16::f32x16;

#[derive(Copy, Clone, Debug)]
pub(crate) struct u8x64(u8x32, u8x32);

arith_ops!(u8x64);

impl Base for u8x64 {}

impl Type for u8x64 {
    type Scalar = u8;
    type Widened = u16x64;
    type Float = f32x16;

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
            // have the full height or full width (i.e. at the very bottom or very right)
            // by adapting the below code. However, I'm seeing a significant slowdown in benchmarks
            // when removing above if conditions, so we instead just fallback to scalar packing
            // for for all cases where we are not packing a full 256x4 tile,
            // so that we have the full performance for the general case.
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

                    for (dest, src) in dest_slices.iter_mut().zip(reinterpreted) {
                        let target: &mut [u8; 16] =
                            (&mut dest[dest_idx..][..16]).try_into().unwrap();
                        vst1q_u8(target.as_mut_ptr(), src)
                    }
                }
            }
        }
    }

    #[inline(always)]
    fn from_float(f: &[Self::Float]) -> Self {
        let f: &[f32x16; 1] = f.try_into().unwrap();
        let mut stored = [u8x16::splat(0); 4];
        let ordered = [f[0].0.0, f[0].0.1, f[0].1.0, f[0].1.1];

        unsafe {
            for (f, stored) in ordered.iter().zip(stored.iter_mut()) {
                let mulled = vfmaq_f32(vdupq_n_f32(0.5), f.0, vdupq_n_f32(255.0));
                let converted = vmovn_u32(vcvtq_u32_f32(mulled));
                let zipped = vzip_u16(converted, converted);
                let combined = vcombine_u16(zipped.0, zipped.1);
                let moved = vmovn_u16(combined);
                let zipped = vzip_u8(moved, moved);
                stored.0 = vcombine_u8(zipped.0, zipped.1);
            }

            u8x64(u8x32(stored[0], stored[1]), u8x32(stored[2], stored[3]))
        }
    }

    #[inline(always)]
    fn splat_4th_element(mut self) -> Self {
        self.0 = self.0.splat_4th_element();
        self.1 = self.1.splat_4th_element();

        self
    }

    #[inline(always)]
    fn load_alphas_f32(src: &[f32]) -> Self {
        todo!()
    }

    fn load_f32_many(src: &[f32]) -> Self {
        todo!()
    }

    const IS_FLOAT: bool = false;
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
