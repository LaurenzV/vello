mod compose;

use crate::fine2::COLOR_COMPONENTS;
use crate::fine2::FineKernel;
use crate::fine2::lowp::compose::ComposeExt;
use crate::peniko::{BlendMode, Compose, Mix};
use crate::region::Region;
use crate::util::BlendModeExt;
use core::iter;
use vello_common::fearless_simd::*;
use vello_common::paint::PremulColor;
use vello_common::tile::Tile;

#[derive(Clone, Copy, Debug)]
pub struct U8Kernel;

impl<S: Simd> FineKernel<S> for U8Kernel {
    type Numeric = u8;
    type Composite = u8x32<S>;

    #[inline]
    fn extract_color(color: PremulColor) -> [Self::Numeric; 4] {
        color.as_premul_rgba8().to_u8_array()
    }

    // TODO: SIMDify on NEON. ALso make scalar version faster (it was faster in previous main version).
    #[inline(always)]
    fn pack(region: &mut Region<'_>, blend_buf: &[Self::Numeric]) {
        for y in 0..Tile::HEIGHT {
            for (x, pixel) in region
                .row_mut(y)
                .chunks_exact_mut(COLOR_COMPONENTS)
                .enumerate()
            {
                let idx = COLOR_COMPONENTS * (usize::from(Tile::HEIGHT) * x + usize::from(y));
                pixel.copy_from_slice(&blend_buf[idx..][..COLOR_COMPONENTS]);
            }
        }
    }

    // Inlining causes performance degradation
    fn copy_solid(simd: S, target: &mut [Self::Numeric], color: [Self::Numeric; 4]) {
        let color =
            u8x64::block_splat(u32x4::splat(simd, u32::from_ne_bytes(color)).reinterpret_u8());

        for el in target.chunks_exact_mut(64) {
            el.copy_from_slice(&color.val);
        }
    }

    #[inline(always)]
    fn copy_f32_iter(
        simd: S,
        target: &mut [Self::Numeric],
        mut src: impl Iterator<Item = f32x16<S>>,
    ) {
        for el in target.chunks_exact_mut(16) {
            let next = src.next().unwrap();
            let mulled = f32x16::splat(simd, 0.5).madd(next, f32x16::splat(simd, 255.0));

            // TODO: SIMDify
            el.copy_from_slice(&[
                mulled.val[0] as u8,
                mulled.val[1] as u8,
                mulled.val[2] as u8,
                mulled.val[3] as u8,
                mulled.val[4] as u8,
                mulled.val[5] as u8,
                mulled.val[6] as u8,
                mulled.val[7] as u8,
                mulled.val[8] as u8,
                mulled.val[9] as u8,
                mulled.val[10] as u8,
                mulled.val[11] as u8,
                mulled.val[12] as u8,
                mulled.val[13] as u8,
                mulled.val[14] as u8,
                mulled.val[15] as u8,
            ])
        }
    }

    #[inline(always)]
    fn alpha_composite_solid(simd: S, target: &mut [Self::Numeric], color: [Self::Numeric; 4]) {
        fill::alpha_composite_solid(simd, target, color);
    }

    fn alpha_composite_shader(simd: S, target: &mut [Self::Numeric], shader_src: &[Self::Numeric]) {
        let src_iter = shader_src
            .chunks_exact(32)
            .map(|el| u8x32::from_slice(simd, el));

        fill::alpha_composite_arbitrary(simd, target, src_iter);
    }

    fn blend(
        simd: S,
        target: &mut [Self::Numeric],
        src: impl Iterator<Item = Self::Composite>,
        blend_mode: BlendMode,
    ) {
        fill::blend(simd, target, src, blend_mode);
    }

    #[inline(always)]
    fn alpha_composite_solid_with_alphas(
        simd: S,
        target: &mut [Self::Numeric],
        color: [Self::Numeric; 4],
        alphas: &[u8],
    ) {
        strip::alpha_composite_solid(simd, target, color, alphas);
    }

    fn alpha_composite_shader_with_alphas(
        simd: S,
        target: &mut [Self::Numeric],
        shader_src: &[Self::Numeric],
        alphas: &[u8],
    ) {
        strip::alpha_composite_arbitrary(
            simd,
            target,
            shader_src
                .chunks_exact(32)
                .map(|el| u8x32::from_slice(simd, el)),
            alphas,
        );
    }

    fn blend_with_alphas(
        simd: S,
        target: &mut [Self::Numeric],
        src: impl Iterator<Item = Self::Composite>,
        blend_mode: BlendMode,
        alphas: &[u8],
    ) {
        strip::blend(simd, target, src, blend_mode, alphas)
    }
}

mod fill {
    use crate::fine2::Splat4thExt;
    use crate::fine2::lowp::compose::ComposeExt;
    use crate::peniko::BlendMode;
    use crate::util::normalized_mul;
    use vello_common::fearless_simd::*;

    pub(super) fn blend<S: Simd, T: Iterator<Item = u8x32<S>>>(
        simd: S,
        target: &mut [u8],
        src_c: T,
        blend_mode: BlendMode,
    ) {
        let mask = u8x32::splat(simd, 255);
        for (part, src_c) in target.chunks_exact_mut(32).zip(src_c) {
            let bg = u8x32::from_slice(simd, part);
            let res = blend_mode.compose(simd, src_c, bg, mask);
            part.copy_from_slice(&res.val);
        }
    }

    #[inline(always)]
    pub(super) fn alpha_composite_solid<S: Simd>(s: S, target: &mut [u8], src_c: [u8; 4]) {
        let one_minus_alpha = 255 - u8x32::splat(s, src_c[3]);
        let src_c = u32x8::splat(s, u32::from_ne_bytes(src_c)).reinterpret_u8();

        for part in target.chunks_exact_mut(64) {
            // We process in batches of 64 because loading/storing is much faster this way (at least on NEON),
            // but since we widen to u16, we can only work with 256 bits, so we split it up.
            let bg = u8x64::from_slice(s, part);
            let (bg_1, bg_2) = s.split_u8x64(bg);
            let res_1 = alpha_composite_inner(s, bg_1, src_c, one_minus_alpha);
            let res_2 = alpha_composite_inner(s, bg_2, src_c, one_minus_alpha);
            let combined = s.combine_u8x32(res_1, res_2);
            part.copy_from_slice(&combined.val);
        }
    }

    pub(super) fn alpha_composite_arbitrary<S: Simd, T: Iterator<Item = u8x32<S>>>(
        simd: S,
        target: &mut [u8],
        src_c: T,
    ) {
        for (part, src_c) in target.chunks_exact_mut(32).zip(src_c) {
            let one_minus_alpha = 255 - src_c.splat_4th();
            let bg = u8x32::from_slice(simd, part);
            let res = alpha_composite_inner(simd, bg, src_c, one_minus_alpha);
            part.copy_from_slice(&res.val);
        }
    }

    #[inline(always)]
    fn alpha_composite_inner<S: Simd>(
        s: S,
        bg: u8x32<S>,
        src_c: u8x32<S>,
        one_minus_alpha: u8x32<S>,
    ) -> u8x32<S> {
        s.narrow_u16x32(normalized_mul(bg, one_minus_alpha)) + src_c
    }
}

mod strip {
    use crate::fine2::Splat4thExt;
    use crate::fine2::lowp::compose::ComposeExt;
    use crate::fine2::lowp::extract_masks;
    use crate::peniko::BlendMode;
    use crate::util::{Div255Ext, normalized_mul};
    use vello_common::fearless_simd::*;

    pub(super) fn blend<S: Simd, T: Iterator<Item = u8x32<S>>>(
        simd: S,
        target: &mut [u8],
        src_c: T,
        blend_mode: BlendMode,
        alphas: &[u8],
    ) {
        for ((bg_part, masks), src_c) in target
            .chunks_exact_mut(32)
            .zip(alphas.chunks_exact(8))
            .zip(src_c)
        {
            let bg = u8x32::from_slice(simd, bg_part);
            let masks = extract_masks(simd, masks);
            let res = blend_mode.compose(simd, src_c, bg, masks);
            bg_part.copy_from_slice(&res.val);
        }
    }

    #[inline(always)]
    pub(super) fn alpha_composite_solid<S: Simd>(
        s: S,
        target: &mut [u8],
        src_c: [u8; 4],
        alphas: &[u8],
    ) {
        let src_a = u8x32::splat(s, src_c[3]);
        let src_c = u32x8::splat(s, u32::from_ne_bytes(src_c)).reinterpret_u8();
        let one = u8x32::splat(s, 255);

        for (bg_part, masks) in target.chunks_exact_mut(32).zip(alphas.chunks_exact(8)) {
            alpha_composite_inner(s, bg_part, masks, src_c, src_a, one);
        }
    }

    #[inline(always)]
    pub(super) fn alpha_composite_arbitrary<S: Simd, T: Iterator<Item = u8x32<S>>>(
        simd: S,
        target: &mut [u8],
        src_c: T,
        alphas: &[u8],
    ) {
        let one = u8x32::splat(simd, 255);

        for ((bg_part, masks), src_c) in target
            .chunks_exact_mut(32)
            .zip(alphas.chunks_exact(8))
            .zip(src_c)
        {
            let src_a = src_c.splat_4th();
            alpha_composite_inner(simd, bg_part, masks, src_c, src_a, one);
        }
    }

    #[inline(always)]
    fn alpha_composite_inner<S: Simd>(
        s: S,
        target: &mut [u8],
        masks: &[u8],
        src_c: u8x32<S>,
        src_a: u8x32<S>,
        one: u8x32<S>,
    ) {
        let bg_c = u8x32::from_slice(s, target);

        let mask_a = extract_masks(s, masks);
        let inv_src_a_mask_a = one - s.narrow_u16x32(normalized_mul(src_a, mask_a));

        let p1 = s.widen_u8x32(bg_c) * s.widen_u8x32(inv_src_a_mask_a);
        let p2 = s.widen_u8x32(src_c) * s.widen_u8x32(mask_a);
        let res = s.narrow_u16x32((p1 + p2).div_255());
        target.copy_from_slice(&res.val);
    }
}

#[inline(always)]
fn extract_masks<S: Simd>(simd: S, masks: &[u8]) -> u8x32<S> {
    let m1 =
        u32x4::splat(simd, u32::from_ne_bytes(masks[0..4].try_into().unwrap())).reinterpret_u8();
    let m2 =
        u32x4::splat(simd, u32::from_ne_bytes(masks[4..8].try_into().unwrap())).reinterpret_u8();

    let zipped1 = m1.zip_low(m1);
    let zipped1 = zipped1.zip_low(zipped1);

    let zipped2 = m2.zip_low(m2);
    let zipped2 = zipped2.zip_low(zipped2);

    simd.combine_u8x16(zipped1, zipped2)
}
