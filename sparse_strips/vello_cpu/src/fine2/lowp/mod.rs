use vello_common::fearless_simd::*;
use vello_common::paint::PremulColor;
use vello_common::tile::Tile;
use crate::fine2::FineKernel;
use crate::fine::COLOR_COMPONENTS;
use crate::Level;
use crate::peniko::BlendMode;
use crate::region::Region;

#[derive(Clone, Copy, Debug)]
pub struct U8Kernel;

impl FineKernel for U8Kernel {
    type Numeric = u8;

    #[inline]
    fn extract_color(color: PremulColor) -> [Self::Numeric; 4] {
        color.as_premul_rgba8().to_u8_array()
    }

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
    fn fill_buf<S: Simd>(simd: S, target: &mut [Self::Numeric], color: [Self::Numeric; 4]) {
        let color = u8x64::block_splat(u32x4::splat(simd, u32::from_ne_bytes(color)).reinterpret_u8());

        for el in target.chunks_exact_mut(64) {
            el.copy_from_slice(&color.val);
        }
    }

    #[inline(always)]
    fn fill_solid<S: Simd>(simd: S, target: &mut [Self::Numeric], color: [Self::Numeric; 4], blend_mode: BlendMode) {
        fill::alpha_composite_solid(simd, target, color);
    }

    #[inline(always)]
    fn strip_solid<S: Simd>(simd: S, target: &mut [Self::Numeric], color: [Self::Numeric; 4], blend_mode: BlendMode, alphas: &[u8]) {
        strip::alpha_composite_solid(simd, target, color, alphas);       
    }
}

mod fill {
    use vello_common::fearless_simd::*;
    use crate::util::{normalized_mul, Div255Ext};
    
    #[inline(always)]
    pub(super) fn alpha_composite_solid<S: Simd>(s: S, target: &mut [u8], src_c: [u8; 4]) {
        let one_minus_alpha = 255 - u8x32::splat(s, src_c[3]);
        let src_c = u32x8::splat(s, u32::from_ne_bytes(src_c)).reinterpret_u8();

        for part in target.chunks_exact_mut(64) {
            alpha_composite_inner(s, part, src_c, one_minus_alpha);
        }
    }

    #[inline(always)]
    fn alpha_composite_inner<S: Simd>(s: S, target: &mut [u8], src_c: u8x32<S>, one_minus_alpha: u8x32<S>) {
        // We process in batches of 64 because loading/storing is much faster this way (at least on NEON),
        // but since we widen to u16, we can only work with 256 bits, so we split it up.
        let bg = u8x64::from_slice(s, target);
        let (bg_1, bg_2) = s.split_u8x64(bg);
        let res_1 = s.narrow_u16x32(normalized_mul(bg_1, one_minus_alpha)) + src_c;
        let res_2 = s.narrow_u16x32(normalized_mul(bg_2, one_minus_alpha)) + src_c;
        let res = s.combine_u8x32(res_1, res_2);
        
        target.copy_from_slice(&res.val)
    }
}

mod strip {
    use vello_common::fearless_simd::*;
    use crate::util::{normalized_mul, Div255Ext};
    
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

        for (bg_part, masks) in target
            .chunks_exact_mut(32)
            .zip(alphas.chunks_exact(8))
        {
            alpha_composite_inner(s, bg_part, masks, src_c, src_a, one);
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
        
        let mask_a = {
            let m1 = u32x4::splat(s, u32::from_ne_bytes(masks[0..4].try_into().unwrap())).reinterpret_u8();
            let m2 = u32x4::splat(s, u32::from_ne_bytes(masks[4..8].try_into().unwrap())).reinterpret_u8();

            let zipped1 = m1.zip1(m1);
            let zipped1 = zipped1.zip1(zipped1);

            let zipped2 = m2.zip1(m2);
            let zipped2 = zipped2.zip1(zipped2);

            s.combine_u8x16(zipped1, zipped2)
        };
        let inv_src_a_mask_a = one - s.narrow_u16x32(normalized_mul(src_a, mask_a));

        let p1 = s.widen_u8x32(bg_c) * s.widen_u8x32(inv_src_a_mask_a);
        let p2 = s.widen_u8x32(src_c) * s.widen_u8x32(mask_a);
        let res = s.narrow_u16x32((p1 + p2).div_255());
        target.copy_from_slice(&res.val);
    }
}
