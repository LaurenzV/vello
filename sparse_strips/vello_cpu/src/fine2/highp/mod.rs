use vello_common::paint::PremulColor;
use vello_common::tile::Tile;
use crate::fine2::FineKernel;
use crate::fine::COLOR_COMPONENTS;
use crate::Level;
use crate::peniko::BlendMode;
use crate::region::Region;

pub struct F32Kernel;

impl FineKernel for F32Kernel {
    type Numeric = f32;

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
    
    #[inline(always)]
    fn fill_buf(level: Level, target: &mut [Self::Numeric], color: [Self::Numeric; 4]) {
        fill::fill_buf(level, target, color);
    }
    
    #[inline(always)]
    fn fill_solid(level: Level, target: &mut [Self::Numeric], color: [Self::Numeric; 4], blend_mode: BlendMode) {
        fill::alpha_composite_solid(level, target, color);
    }

    #[inline(always)]
    fn strip_solid(level: Level, target: &mut [Self::Numeric], color: [Self::Numeric; 4], blend_mode: BlendMode, alphas: &[u8]) {
        strip::alpha_composite_solid(level, target, color, alphas);
    }
}

mod fill {
    use vello_common::fearless_simd::*;
    use crate::util::normalized_mul;

    // Careful: From my experiments, inlining these functions can have drastic (negative)
    // consequences on performance.

    simd_dispatch!(pub(super) fill_buf(level, target: &mut [f32], src_c: [f32; 4]) = fill_buf_dispatch);

    pub(super) fn fill_buf_dispatch<S: Simd>(s: S, target: &mut [f32], color: [f32; 4]) {
        let color = f32x16::block_splat(color.simd_into(s));

        for el in target.chunks_exact_mut(16) {
            el.copy_from_slice(&color.val);
        }
    }

    simd_dispatch!(#[inline(always)] pub(crate) alpha_composite_solid(level, target: &mut [f32], src_c: [f32; 4]) = alpha_composite_solid_dispatch);

    #[inline(always)]
    pub(super) fn alpha_composite_solid_dispatch<S: Simd>(s: S, target: &mut [f32], src_c: [f32; 4]) {
        let one_minus_alpha = f32x16::block_splat(f32x4::splat(s, src_c[3]));
        let src_c = f32x16::block_splat(f32x4::simd_from(src_c, s));

        for part in target.chunks_exact_mut(16) {
            alpha_composite_inner(s, part, src_c, one_minus_alpha);
        }
    }

    #[inline(always)]
    fn alpha_composite_inner<S: Simd>(s: S, target: &mut [f32], src_c: f32x16<S>, one_minus_alpha: f32x16<S>) {
        let mut bg_c = f32x16::from_slice(s, target);
        bg_c = bg_c * one_minus_alpha + src_c;
        target.copy_from_slice(&bg_c.val)
    }
}

mod strip {
    use vello_common::fearless_simd::*;
    use crate::util::normalized_mul;

    simd_dispatch!(#[inline(always)] pub(crate) alpha_composite_solid(level, target: &mut [f32], src_c: [f32; 4], alphas: &[u8]) = alpha_composite_solid_dispatch);

    #[inline(always)]
    fn alpha_composite_solid_dispatch<S: Simd>(
        s: S,
        target: &mut [f32],
        src_c: [f32; 4],
        alphas: &[u8],
    ) {
        let src_a = f32x16::splat(s, src_c[3]);
        let src_c = f32x16::block_splat(src_c.simd_into(s));
        let one = f32x16::splat(s, 1.0);

        for (bg_part, masks) in target
            .chunks_exact_mut(16)
            .zip(alphas.chunks_exact(4))
        {
            alpha_composite_inner(s, bg_part, masks, src_c, src_a, one);
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
            // TODO: Use SIMD
            let base_mask = [
                masks[0] as f32 / 255.0,
                masks[1] as f32 / 255.0,
                masks[2] as f32 / 255.0,
                masks[3] as f32 / 255.0,
            ].simd_into(s);
            
            let res = f32x16::block_splat(base_mask);
            let zip1 = res.zip(res).0;
            let zip2 = zip1.zip(zip1).0;
            
            zip2
        };
        let inv_src_a_mask_a = one - (src_a * mask_a);

    
        let res = (bg_c * inv_src_a_mask_a) + (src_c * mask_a);
        target.copy_from_slice(&res.val);
    }
}