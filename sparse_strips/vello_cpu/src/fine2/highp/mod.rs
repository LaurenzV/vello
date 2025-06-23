use crate::fine::COLOR_COMPONENTS;
use crate::fine2::FineKernel;
use crate::kurbo::{Point, Vec2};
use crate::peniko::BlendMode;
use crate::region::Region;
use vello_common::fearless_simd::*;
use vello_common::paint::PremulColor;
use vello_common::tile::Tile;

pub(crate) mod gradient;
pub(crate) mod rounded_blurred_rect;
pub(crate) mod compose;

#[derive(Clone, Copy, Debug)]
pub struct F32Kernel;

impl<S: Simd> FineKernel<S> for F32Kernel {
    type Numeric = f32;
    type Composite = f32x16<S>;

    #[inline(always)]
    fn extract_color(color: PremulColor) -> [Self::Numeric; 4] {
        color.as_premul_f32().components
    }

    fn pack(region: &mut Region<'_>, blend_buf: &[Self::Numeric]) {
        for y in 0..Tile::HEIGHT {
            for (x, pixel) in region
                .row_mut(y)
                .chunks_exact_mut(COLOR_COMPONENTS)
                .enumerate()
            {
                let idx = COLOR_COMPONENTS * (usize::from(Tile::HEIGHT) * x + usize::from(y));
                let start = &blend_buf[idx..];
                // TODO: Use SIMD
                let converted = [
                    (start[0] * 255.0 + 0.5) as u8,
                    (start[1] * 255.0 + 0.5) as u8,
                    (start[2] * 255.0 + 0.5) as u8,
                    (start[3] * 255.0 + 0.5) as u8,
                ];
                pixel.copy_from_slice(&converted);
            }
        }
    }

    // Not having this tanks performance for some reason.
    #[inline(never)]
    fn copy_solid(simd: S, target: &mut [Self::Numeric], color: [Self::Numeric; 4]) {
        let color = f32x16::block_splat(color.simd_into(simd));

        for el in target.chunks_exact_mut(16) {
            el.copy_from_slice(&color.val);
        }
    }

    #[inline(always)]
    fn copy_f32_iter(
        _: S,
        target: &mut [Self::Numeric],
        mut src: impl Iterator<Item = f32x16<S>>,
    ) {
        for el in target.chunks_exact_mut(16) {
            el.copy_from_slice(&src.next().unwrap().val);
        }
    }

    #[inline(always)]
    fn alpha_composite_solid(
        simd: S,
        target: &mut [Self::Numeric],
        color: [Self::Numeric; 4],
    ) {
        fill::alpha_composite_solid(simd, target, color);
    }

    fn alpha_composite_shader(
        simd: S,
        target: &mut [Self::Numeric],
        shader_src: &[Self::Numeric],
    ) {
        fill::alpha_composite_arbitrary(
            simd,
            target,
            shader_src
                .chunks_exact(16)
                .map(|el| f32x16::from_slice(simd, el)),
        );
    }

    fn blend(simd: S, target: &mut [Self::Numeric], src: impl Iterator<Item=Self::Composite>, _: BlendMode) {
        // TODO: Impl blend
        fill::alpha_composite_arbitrary(
            simd,
            target,
            src,
        );
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
                .chunks_exact(16)
                .map(|el| f32x16::from_slice(simd, el)),
            alphas,
        );
    }

    fn blend_with_alphas(simd: S, target: &mut [Self::Numeric], src: impl Iterator<Item=Self::Composite>, _: BlendMode, alphas: &[u8]) {
        strip::alpha_composite_arbitrary(
            simd,
            target,
            src,
            alphas,
        );
    }
}

mod fill {
    use crate::fine2::Splat4thExt;
    use vello_common::fearless_simd::*;
    // Careful: From my experiments, inlining these functions can have drastic (negative)
    // consequences on performance.

    #[inline(always)]
    pub(super) fn alpha_composite_solid<S: Simd>(s: S, target: &mut [f32], src_c: [f32; 4]) {
        let one_minus_alpha = f32x16::block_splat(f32x4::splat(s, src_c[3]));
        let src_c = f32x16::block_splat(f32x4::simd_from(src_c, s));

        for part in target.chunks_exact_mut(16) {
            alpha_composite_inner(s, part, src_c, one_minus_alpha);
        }
    }

    #[inline(always)]
    pub(super) fn alpha_composite_arbitrary<S: Simd, T: Iterator<Item = f32x16<S>>>(
        simd: S,
        target: &mut [f32],
        src_c: T,
    ) {
        for (part, src_c) in target.chunks_exact_mut(16).zip(src_c) {
            let one_minus_alpha = 1.0 - src_c.splat_4th();
            alpha_composite_inner(simd, part, src_c, one_minus_alpha)
        }
    }

    #[inline(always)]
    fn alpha_composite_inner<S: Simd>(
        s: S,
        target: &mut [f32],
        src_c: f32x16<S>,
        one_minus_alpha: f32x16<S>,
    ) {
        let mut bg_c = f32x16::from_slice(s, target);
        bg_c = src_c.madd(one_minus_alpha, bg_c);
        target.copy_from_slice(&bg_c.val)
    }
}

mod strip {
    use crate::fine2::Splat4thExt;
    use crate::util::normalized_mul;
    use vello_common::fearless_simd::*;

    #[inline(always)]
    pub(super) fn alpha_composite_solid<S: Simd>(
        s: S,
        target: &mut [f32],
        src_c: [f32; 4],
        alphas: &[u8],
    ) {
        let src_a = f32x16::splat(s, src_c[3]);
        let src_c = f32x16::block_splat(src_c.simd_into(s));
        let one = f32x16::splat(s, 1.0);

        for (bg_part, masks) in target.chunks_exact_mut(16).zip(alphas.chunks_exact(4)) {
            alpha_composite_inner(s, bg_part, masks, src_c, src_a, one);
        }
    }

    #[inline(always)]
    pub(super) fn alpha_composite_arbitrary<S: Simd, T: Iterator<Item = f32x16<S>>>(
        simd: S,
        target: &mut [f32],
        src_c: T,
        alphas: &[u8],
    ) {
        let one = f32x16::splat(simd, 1.0);

        for ((bg_part, masks), src_c) in target
            .chunks_exact_mut(16)
            .zip(alphas.chunks_exact(4))
            .zip(src_c)
        {
            let src_a = src_c.splat_4th();
            alpha_composite_inner(simd, bg_part, masks, src_c, src_a, one);
        }
    }

    #[inline(always)]
    fn alpha_composite_inner<S: Simd>(
        s: S,
        target: &mut [f32],
        masks: &[u8],
        src_c: f32x16<S>,
        src_a: f32x16<S>,
        one: f32x16<S>,
    ) {
        let bg_c = f32x16::from_slice(s, target);

        let mask_a = {
            let mut base_mask = [
                masks[0] as f32,
                masks[1] as f32,
                masks[2] as f32,
                masks[3] as f32,
            ]
            .simd_into(s);

            base_mask = base_mask * f32x4::splat(s, 1.0 / 255.0);

            let res = f32x16::block_splat(base_mask);
            let zip_low = res.zip_low(res);
            let zip_high = zip_low.zip_low(zip_low);

            zip_high
        };

        let inv_src_a_mask_a = one.msub(src_a, mask_a);

        let res = (src_c * mask_a).madd(bg_c, inv_src_a_mask_a);
        target.copy_from_slice(&res.val);
    }
}

#[inline(always)]
fn calc_pos(start_pos: Point, idx: usize, x_advance: Vec2, y_advance: Vec2) -> Point {
    let col_idx = idx >> (Tile::HEIGHT.trailing_zeros() as usize);
    let row_idx = idx & (Tile::HEIGHT as usize - 1);

    start_pos + (x_advance * col_idx as f64 + y_advance * row_idx as f64)
}

#[inline(always)]
fn element_wise_splat<S: Simd>(simd: S, input: f32x4<S>) -> f32x16<S> {
    simd.combine_f32x8(
        simd.combine_f32x4(
            f32x4::splat(simd, input.val[0]),
            f32x4::splat(simd, input.val[1]),
        ),
        simd.combine_f32x4(
            f32x4::splat(simd, input.val[2]),
            f32x4::splat(simd, input.val[3]),
        ),
    )
}
