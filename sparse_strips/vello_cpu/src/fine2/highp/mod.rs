use vello_common::paint::PremulColor;
use vello_common::tile::Tile;
use crate::fine2::FineKernel;
use crate::fine::COLOR_COMPONENTS;
use crate::Level;
use crate::peniko::BlendMode;
use crate::region::Region;

pub(crate) struct F32Kernel;

impl FineKernel for F32Kernel {
    type Numeric = f32;

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


    fn fill_solid(level: Level, target: &mut [Self::Numeric], color: [Self::Numeric; 4], blend_mode: BlendMode) {
        fill::alpha_composite_solid(level, target, color);
    }
}

mod fill {
    use vello_common::fearless_simd::*;
    use crate::util::normalized_mul;

    simd_dispatch!(pub(crate) alpha_composite_solid(level, target: &mut [f32], src_c: [f32; 4]) = alpha_composite_solid_dispatch);

    #[inline(always)]
    pub(super) fn alpha_composite_solid_dispatch<S: Simd>(s: S, target: &mut [f32], src_c: [f32; 4]) {
        let one_minus_alpha = f32x8::block_splat(f32x4::splat(s, src_c[3]));
        let src_c = f32x8::block_splat(f32x4::simd_from(src_c, s));

        for part in target.chunks_exact_mut(8) {
            alpha_composite_inner(s, part, src_c, one_minus_alpha);
        }
    }

    #[inline(always)]
    fn alpha_composite_inner<S: Simd>(s: S, target: &mut [f32], src_c: f32x8<S>, one_minus_alpha: f32x8<S>) {
        let mut bg_c = f32x8::from_slice(s, target);
        bg_c = bg_c * one_minus_alpha + src_c;
        target.copy_from_slice(&bg_c.val)
    }
}