use vello_common::paint::PremulColor;
use vello_common::tile::Tile;
use crate::fine2::FineKernel;
use crate::fine::COLOR_COMPONENTS;
use crate::Level;
use crate::peniko::BlendMode;
use crate::region::Region;

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
    
    fn fill_solid(level: Level, target: &mut [Self::Numeric], color: [Self::Numeric; 4], blend_mode: BlendMode) {
        fill::alpha_composite_solid(level, target, color);
    }

    fn strip_solid(level: Level, target: &mut [Self::Numeric], color: [Self::Numeric; 4], blend_mode: BlendMode, alphas: &[u8]) {
        strip::alpha_composite_solid(level, target, color, alphas);       
    }
}

mod fill {
    use vello_common::fearless_simd::*;
    use crate::util::{normalized_mul, Div255Ext};
    use crate::Level;

    simd_dispatch!(pub(crate) alpha_composite_solid(level, target: &mut [u8], src_c: [u8; 4]) = alpha_composite_solid_dispatch);

    #[inline(always)]
    pub(super) fn alpha_composite_solid_dispatch<S: Simd>(s: S, target: &mut [u8], src_c: [u8; 4]) {
        let one_minus_alpha = 255 - u8x32::splat(s, src_c[3]);
        let src_c = u32x8::splat(s, u32::from_ne_bytes(src_c)).reinterpret_u8();

        for part in target.chunks_exact_mut(32) {
            alpha_composite_inner(s, part, src_c, one_minus_alpha);
        }
    }

    #[inline(always)]
    fn alpha_composite_inner<S: Simd>(s: S, target: &mut [u8], src_c: u8x32<S>, one_minus_alpha: u8x32<S>) {
        let bg_c = u8x32::from_slice(s, target);
        let res = s.narrow_u16x32(normalized_mul(bg_c, one_minus_alpha)) + src_c;
        target.copy_from_slice(&res.val)
    }
}

mod strip {
    use vello_common::fearless_simd::*;
    use crate::util::{normalized_mul, Div255Ext};

    simd_dispatch!(pub(crate) alpha_composite_solid(level, target: &mut [u8], src_c: [u8; 4], alphas: &[u8]) = alpha_composite_solid_dispatch);

    #[inline(always)]
    fn alpha_composite_solid_dispatch<S: Simd>(
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

            let zipped1 = m1.zip(m1).0;
            let zipped1 = zipped1.zip(zipped1);

            let zipped2 = m2.zip(m2).0;
            let zipped2 = zipped2.zip(zipped2);

            s.combine_u8x16(zipped1.0, zipped2.0)
        };
        let inv_src_a_mask_a = one - s.narrow_u16x32(normalized_mul(src_a, mask_a));

        let p1 = s.widen_u8x32(bg_c) * s.widen_u8x32(inv_src_a_mask_a);
        let p2 = s.widen_u8x32(src_c) * s.widen_u8x32(mask_a);
        let res = s.narrow_u16x32((p1 + p2).div_255());
        target.copy_from_slice(&res.val);
    }
}
