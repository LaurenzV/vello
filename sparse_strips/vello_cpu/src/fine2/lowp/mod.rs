use vello_common::paint::PremulColor;
use vello_common::tile::Tile;
use crate::fine2::FineKernel;
use crate::fine::COLOR_COMPONENTS;
use crate::Level;
use crate::peniko::BlendMode;
use crate::region::Region;

pub(crate) struct U8Kernel;

impl FineKernel for U8Kernel {
    type Numeric = u8;

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
}

mod fill {
    use vello_common::fearless_simd::*;
    use crate::util::normalized_mul;
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
        let mut bg_c = u8x32::from_slice(s, target);
        bg_c = s.narrow_u16x32(normalized_mul(bg_c, one_minus_alpha)) + src_c;
        target.copy_from_slice(&bg_c.val)
    }
}
